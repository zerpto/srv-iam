"use client";

import { Checkbox } from "@headlessui/react";
import { CheckIcon, MinusIcon } from "@heroicons/react/16/solid";
import { useTranslations } from "next-intl";
import { useMemo, useState } from "react";
import { SearchField } from "@/components/ui/search-field";
import { usePermissionCatalog } from "@/lib/roles";

const boxClass =
  "group grid size-4.5 shrink-0 cursor-pointer place-items-center rounded-[5px] bg-white ring-1 ring-zinc-300 ring-inset transition-colors data-checked:bg-brand-600 data-checked:ring-brand-600 data-indeterminate:bg-brand-600 data-indeterminate:ring-brand-600 data-disabled:cursor-not-allowed data-disabled:opacity-50 data-focus:outline-2 data-focus:outline-offset-2 data-focus:outline-brand-600 dark:bg-white/5 dark:ring-white/20";

/**
 * Pick permissions from the catalogue, grouped by module with a select-all per module.
 * `locked` permissions can't be changed (the caller doesn't hold them, so can't grant them).
 */
export function PermissionPicker({
  value,
  onChange,
  readOnly,
  isLocked,
}: {
  value: string[];
  onChange: (value: string[]) => void;
  readOnly?: boolean;
  isLocked?: (key: string) => boolean;
}) {
  const t = useTranslations("roles");
  const catalog = usePermissionCatalog();
  const [query, setQuery] = useState("");
  const selected = useMemo(() => new Set(value), [value]);

  const modules = useMemo(() => {
    const needle = query.trim().toLowerCase();
    if (!needle) return catalog.modules;
    return catalog.modules
      .map(({ module, items }) => ({
        module,
        items: items.filter((p) => `${p.key} ${p.description}`.toLowerCase().includes(needle)),
      }))
      .filter(({ items }) => items.length > 0);
  }, [catalog.modules, query]);

  function toggle(keys: string[], on: boolean) {
    const next = new Set(selected);
    for (const key of keys) {
      if (isLocked?.(key)) continue;
      if (on) next.add(key);
      else next.delete(key);
    }
    onChange([...next]);
  }

  if (catalog.isPending) {
    return (
      <div aria-busy="true" className="h-64 animate-pulse rounded-xl bg-zinc-100 dark:bg-white/5" />
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <SearchField
          value={query}
          onChange={setQuery}
          label={t("searchPermissions")}
          placeholder={t("searchPermissions")}
        />
        <p className="text-sm text-zinc-500 dark:text-zinc-400">
          {t("selectedCount", { count: selected.size, total: catalog.permissions.length })}
        </p>
      </div>

      {modules.length === 0 && (
        <p className="py-6 text-center text-sm text-zinc-500 dark:text-zinc-400">
          {t("noPermissionMatch")}
        </p>
      )}

      {modules.map(({ module, items }) => {
        const editable = items.filter((p) => !isLocked?.(p.key)).map((p) => p.key);
        const chosen = items.filter((p) => selected.has(p.key)).length;
        const all = chosen === items.length;
        return (
          <section key={module} className="rounded-xl ring-1 ring-zinc-200 dark:ring-white/10">
            <header className="flex items-center gap-3 border-b border-zinc-200 bg-zinc-50 px-4 py-2.5 dark:border-white/10 dark:bg-white/5">
              <Checkbox
                aria-label={t("toggleModule", { module: module.toUpperCase() })}
                checked={all}
                indeterminate={!all && chosen > 0}
                disabled={readOnly || editable.length === 0}
                onChange={(on) => toggle(editable, on)}
                className={boxClass}
              >
                <CheckIcon
                  aria-hidden="true"
                  className="hidden size-3.5 text-white group-data-checked:block"
                />
                <MinusIcon
                  aria-hidden="true"
                  className="hidden size-3.5 text-white group-data-indeterminate:block"
                />
              </Checkbox>
              <h3 className="text-xs font-semibold tracking-wide text-zinc-700 uppercase dark:text-zinc-300">
                {module}
              </h3>
              <span className="ml-auto text-xs text-zinc-500 tabular-nums dark:text-zinc-400">
                {chosen}/{items.length}
              </span>
            </header>
            <ul className="grid gap-x-6 gap-y-1 p-3 sm:grid-cols-2">
              {items.map((permission) => {
                const locked = Boolean(isLocked?.(permission.key));
                return (
                  <li key={permission.key}>
                    <label className="flex cursor-pointer items-start gap-3 rounded-lg px-2 py-1.5 hover:bg-zinc-50 dark:hover:bg-white/5">
                      <Checkbox
                        checked={selected.has(permission.key)}
                        disabled={readOnly || locked}
                        onChange={(on) => toggle([permission.key], on)}
                        className={`${boxClass} mt-0.5`}
                      >
                        <CheckIcon
                          aria-hidden="true"
                          className="size-3.5 text-white opacity-0 group-data-checked:opacity-100"
                        />
                      </Checkbox>
                      <span className="min-w-0">
                        <span className="block truncate font-mono text-xs text-zinc-900 dark:text-white">
                          {permission.key}
                        </span>
                        <span className="block text-xs text-zinc-500 dark:text-zinc-400">
                          {permission.description}
                        </span>
                      </span>
                    </label>
                  </li>
                );
              })}
            </ul>
          </section>
        );
      })}
    </div>
  );
}
