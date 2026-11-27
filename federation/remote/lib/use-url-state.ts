"use client";

import { useSearchParams } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import type { SortItem } from "@/components/ui/data-table";

/**
 * List filters, search and sort live in the URL so a filtered list can be copied, bookmarked or
 * linked to. Writes use `history.replaceState` (no history entry per keystroke, no server round
 * trip); Next.js syncs `useSearchParams` with it. Other params (`?tab=`, `?from=`) are kept, and
 * a value equal to its default is removed to keep the URL short.
 */
function writeParams(updates: Record<string, string | null>) {
  const next = new URLSearchParams(window.location.search);
  for (const [key, value] of Object.entries(updates)) {
    if (value) next.set(key, value);
    else next.delete(key);
  }
  const query = next.toString();
  window.history.replaceState(
    null,
    "",
    `${window.location.pathname}${query ? `?${query}` : ""}${window.location.hash}`,
  );
}

/** Removes the given params at once, e.g. a "Clear filters" button. */
export function useClearUrlParams() {
  return useCallback((keys: string[]) => {
    writeParams(Object.fromEntries(keys.map((key) => [key, null])));
  }, []);
}

/**
 * Like `useState` for a string filter, stored in `?key=`. `allowed` rejects values the URL may
 * carry that the page doesn't know (they fall back to `fallback`).
 */
export function useUrlParam<T extends string = string>(
  key: string,
  fallback: NoInfer<T>,
  allowed?: readonly NoInfer<T>[],
): [T, (value: T) => void] {
  const raw = useSearchParams().get(key);
  const value = (raw && (!allowed || allowed.includes(raw as T)) ? raw : fallback) as T;
  const set = useCallback(
    (next: T) => writeParams({ [key]: next === fallback ? null : next }),
    [key, fallback],
  );
  return [value, set];
}

/** A boolean filter (a checkbox or switch), stored as `?key=1`. */
export function useUrlFlag(key: string): [boolean, (value: boolean) => void] {
  const on = useSearchParams().get(key) === "1";
  const set = useCallback((next: boolean) => writeParams({ [key]: next ? "1" : null }), [key]);
  return [on, set];
}

/**
 * The text of a search box, stored in `?key=`. Typing stays in local state so the input never
 * lags behind the URL; the URL follows, and a change made elsewhere (a link, "Clear filters")
 * flows back into the input.
 */
export function useUrlSearch(key = "q"): [string, (value: string) => void] {
  const fromUrl = useSearchParams().get(key) ?? "";
  const [text, setText] = useState(fromUrl);
  const written = useRef(fromUrl);

  useEffect(() => {
    if (fromUrl !== written.current) {
      written.current = fromUrl;
      setText(fromUrl);
    }
  }, [fromUrl]);

  const set = useCallback(
    (next: string) => {
      setText(next);
      written.current = next.trim();
      writeParams({ [key]: next.trim() || null });
    },
    [key],
  );
  return [text, set];
}

/** Multi-column sort for `DataTable`, stored as `?sort=date:desc,total:asc`. */
export function useUrlSort(key = "sort"): [SortItem[], (sort: SortItem[]) => void] {
  const raw = useSearchParams().get(key) ?? "";
  const sort = useMemo(
    () =>
      raw.split(",").flatMap((part): SortItem[] => {
        const [id, direction] = part.split(":");
        return id && (direction === "asc" || direction === "desc") ? [{ id, direction }] : [];
      }),
    [raw],
  );
  const set = useCallback(
    (next: SortItem[]) =>
      writeParams({ [key]: next.map((item) => `${item.id}:${item.direction}`).join(",") || null }),
    [key],
  );
  return [sort, set];
}
