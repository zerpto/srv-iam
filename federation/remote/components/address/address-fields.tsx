"use client";

import { keepPreviousData } from "@tanstack/react-query";
import { useTranslations } from "next-intl";
import { useEffect, useMemo, useState } from "react";
import { ComboboxField } from "@/components/ui/combobox-field";
import { TextField } from "@/components/ui/text-field";
import { useApiQuery } from "@/lib/api/query";
import { sdk } from "@/lib/api/sdk";
import {
  ADDRESS_LEVELS,
  type AddressLevel,
  type AddressValue,
  pickedFrom,
  regionChain,
} from "@/lib/address";
import { useDebouncedValue } from "@/lib/use-debounced-value";

type Props = {
  value: AddressValue;
  onChange: (value: AddressValue) => void;
  /** Error message per field. */
  errors?: Partial<Record<keyof AddressValue, string>>;
};

const ROOT = "root";

/**
 * Address built on the region hierarchy (`GET /v1/ref/regions`). Each level is a searchable dropdown of
 * the regions under the one above. Where there are no regions to choose from (the country has no
 * region data, or the level below the last one that has any), the level becomes a text input and
 * so do all the levels under it.
 */
export function AddressFields({ value, onChange, errors = {} }: Props) {
  const t = useTranslations("address");
  // Regions whose child list came back empty: their children are typed instead.
  const [emptyParents, setEmptyParents] = useState<ReadonlySet<string>>(new Set());
  useRestoreChain(value, onChange);

  const markEmpty = (parentKey: string) =>
    setEmptyParents((current) =>
      current.has(parentKey) ? current : new Set(current).add(parentKey),
    );

  // Walk the levels top-down to decide, for each, whether it is a dropdown, a disabled dropdown
  // (waiting for the level above) or a text input.
  const levels: {
    level: AddressLevel;
    depth: number;
    parent: AddressValue["picked"][number] | null;
    parentKey: string;
    mode: "text" | "select" | "waiting";
  }[] = [];
  let typed = false;
  for (const [index, level] of ADDRESS_LEVELS.entries()) {
    const depth = index + 1;
    const parent = depth === 1 ? null : (value.picked[depth - 2] ?? null);
    const parentKey = depth === 1 ? ROOT : (parent?.id ?? "");
    const waiting = depth > 1 && !parent && !typed;
    typed =
      typed || (depth > 1 && parent ? !parent.hasChildren : false) || emptyParents.has(parentKey);
    levels.push({
      level,
      depth,
      parent,
      parentKey,
      mode: typed ? "text" : waiting ? "waiting" : "select",
    });
  }

  // The country says how deep an address must go. Block submitting while a dropdown is still due.
  const requiredDepth = value.picked[0]?.requiredDepth ?? 0;
  const next = levels[value.picked.length];
  const incompleteLevel: AddressLevel | "" =
    requiredDepth > value.picked.length && next?.mode === "select" ? next.level : "";
  useEffect(() => {
    if (value.incompleteLevel !== incompleteLevel) onChange({ ...value, incompleteLevel });
  });

  function set(patch: Partial<AddressValue>) {
    onChange({ ...value, ...patch });
  }

  function pick(depth: number, region: Parameters<typeof pickedFrom>[0] | null) {
    const keep = value.picked.slice(0, depth - 1);
    const cleared = Object.fromEntries(ADDRESS_LEVELS.slice(depth - 1).map((field) => [field, ""]));
    if (!region) {
      set({ ...cleared, picked: keep, regionId: keep.at(-1)?.id ?? "", postalCode: "" });
      return;
    }
    const picked = pickedFrom(region);
    set({
      ...cleared,
      [ADDRESS_LEVELS[depth - 1]]: depth === 1 ? region.code : region.name,
      picked: [...keep, picked],
      regionId: region.id,
      postalCode: region.postalCode ?? "",
    });
  }

  const postalFromRegion =
    Boolean(value.picked.at(-1)?.postalCode) &&
    value.postalCode === value.picked.at(-1)?.postalCode;

  return (
    <div className="loose:space-y-5 space-y-3.5">
      {levels.map(({ level, depth, parent, parentKey, mode }) =>
        mode === "text" ? (
          <TextField
            key={`${level}:text`}
            label={t(`levels.${level}`)}
            placeholder={t("typePlaceholder", { label: t(`levels.${level}`).toLowerCase() })}
            maxLength={level === "country" ? 2 : undefined}
            hint={level === "country" ? t("countryHint") : undefined}
            error={
              errors[level] ??
              (value.incompleteLevel === level
                ? t("pickLevel", { label: t(`levels.${level}`) })
                : undefined)
            }
            value={value[level]}
            onChange={(event) =>
              set({
                [level]:
                  level === "country" ? event.target.value.toUpperCase() : event.target.value,
              })
            }
          />
        ) : (
          <RegionSelect
            key={`${level}:select`}
            level={level}
            parentId={parent?.id}
            parentKey={parentKey}
            disabled={mode === "waiting"}
            selected={value.picked[depth - 1] ?? null}
            error={errors[level]}
            onPick={(region) => pick(depth, region)}
            onEmpty={markEmpty}
          />
        ),
      )}

      <TextField
        label={t("postalCode")}
        inputMode="numeric"
        hint={postalFromRegion ? t("postalFromRegion") : undefined}
        disabled={postalFromRegion}
        error={errors.postalCode}
        value={value.postalCode}
        onChange={(event) => set({ postalCode: event.target.value })}
      />
      <TextField
        label={t("line1")}
        error={errors.line1}
        value={value.line1}
        onChange={(event) => set({ line1: event.target.value })}
      />
      <TextField
        label={t("line2")}
        value={value.line2}
        onChange={(event) => set({ line2: event.target.value })}
      />
    </div>
  );
}

function RegionSelect({
  level,
  parentId,
  parentKey,
  disabled,
  selected,
  error,
  onPick,
  onEmpty,
}: {
  level: AddressLevel;
  parentId: string | undefined;
  parentKey: string;
  disabled: boolean;
  selected: AddressValue["picked"][number] | null;
  error?: string;
  onPick: (region: Parameters<typeof pickedFrom>[0] | null) => void;
  onEmpty: (parentKey: string) => void;
}) {
  const t = useTranslations("address");
  const tc = useTranslations("common");
  const [search, setSearch] = useState("");
  const q = useDebouncedValue(search, 250) || undefined;
  const query = useApiQuery(
    "/v1/ref/regions",
    { parentId, q, perPage: 100 },
    (request) => sdk.regions.listRegions(request),
    { enabled: !disabled, staleTime: 5 * 60_000, placeholderData: keepPreviousData },
  );
  // A level still waiting for its parent has no `parentId`, so it shares the root query's cache
  // entry (the countries). Ignore that data until the level is enabled.
  const regions = useMemo(() => (disabled ? [] : (query.data?.data ?? [])), [disabled, query.data]);

  // No regions under this one (and no search filtering them out): the level is typed instead.
  useEffect(() => {
    if (!disabled && query.isSuccess && !q && regions.length === 0) onEmpty(parentKey);
  }, [disabled, query.isSuccess, q, regions.length, parentKey, onEmpty]);

  const options = regions.map((region) => ({
    value: region.id,
    label: region.name,
    description: region.postalCode ?? undefined,
  }));
  const label = regions[0]?.levelLabel ?? t(`levels.${level}`);

  return (
    <ComboboxField
      label={label}
      value={selected?.id ?? ""}
      selected={selected ? { value: selected.id, label: selected.name } : null}
      options={options}
      onSearch={setSearch}
      loading={query.isFetching}
      onChange={(id) => onPick(regions.find((region) => region.id === id) ?? null)}
      placeholder={t("pickPlaceholder", { label: label.toLowerCase() })}
      emptyText={tc("noResults")}
      disabled={disabled}
      error={error}
    />
  );
}

/** Rebuilds the picked chain from a saved `regionId` (editing an address). */
function useRestoreChain(value: AddressValue, onChange: (value: AddressValue) => void) {
  const needed = Boolean(value.regionId) && value.picked.length === 0;
  const query = useApiQuery(
    "/v1/ref/regions/{id}",
    { id: value.regionId, include: "parent.parent.parent.parent" },
    (request) => sdk.regions.getRegion(request),
    { enabled: needed, staleTime: 5 * 60_000 },
  );
  useEffect(() => {
    if (needed && query.data) onChange({ ...value, picked: regionChain(query.data) });
  });
}
