import { useInfiniteQuery } from "@tanstack/react-query";
import { useEffect, useMemo } from "react";
import { CURSOR_PATHS, LISTS } from "./lists";
import { queryApi } from "./result";

type Lists = typeof LISTS;

/** Every API path that answers with a paginated `{ data, pagination }` list. */
export type ListPath = keyof Lists;

type ListRequest<P extends ListPath> = NonNullable<Parameters<Lists[P]>[0]>;

type ListPage<P extends ListPath> = Awaited<ReturnType<Lists[P]>>;

/** Query parameters other than paging (e.g. `status`, `customer_id`, `q`). */
export type ListFilters<P extends ListPath> = Omit<
  ListRequest<P>,
  "id" | "limit" | "cursor" | "page" | "perPage"
>;

export type ListRow<P extends ListPath> = ListPage<P> extends { data: (infer R)[] } ? R : never;

type Pagination = {
  page?: number;
  totalPages?: number;
  nextCursor?: string | null;
  hasMore?: boolean;
};

const MAX_PER_PAGE = 100;

/** What `queryApi` returns for a list: the rows plus the paging numbers. */
type ListBodyOf<P extends ListPath> = { data: ListRow<P>[]; pagination: unknown };

/**
 * Paginated list with TanStack Query ("load more" style). Page-numbered lists send
 * `page`/`perPage`; the few cursor lists send `cursor`/`limit` (TanStack's infinite queries have no
 * built-in cursor handling for the spec's `?cursor=` lists). The query key starts
 * with `["get", path]`, so `invalidateQueries({ queryKey: ["get", path] })` refreshes it. `rows`
 * is every loaded page, flattened.
 */
export function useCursorList<P extends ListPath>(
  path: P,
  {
    limit = 25,
    filters,
    enabled = true,
    pathParams,
  }: {
    limit?: number;
    filters?: ListFilters<P>;
    enabled?: boolean;
    /** Values for `{id}`-style segments, e.g. `/v1/bank/statements/{id}/lines`. */
    pathParams?: Record<string, string>;
  } = {},
) {
  const byCursor = CURSOR_PATHS.has(path);
  const list = LISTS[path] as (request: object, init?: RequestInit) => Promise<unknown>;
  const size = Math.min(limit, MAX_PER_PAGE);
  const query = useInfiniteQuery({
    queryKey: [
      "get",
      path,
      {
        params: {
          ...(pathParams && { path: pathParams }),
          query: byCursor ? { limit: size, ...filters } : { perPage: size, ...filters },
        },
      },
      "infinite",
    ],
    // A cursor, or a page number for page-numbered lists. `null` is the first page.
    initialPageParam: null as string | number | null,
    enabled,
    queryFn: async ({ pageParam, signal }) => {
      const paging = byCursor
        ? { limit: size, ...(pageParam ? { cursor: pageParam } : {}) }
        : { perPage: size, page: pageParam ?? 1 };
      const request = { ...pathParams, ...filters, ...paging };
      // The path union can't satisfy the SDK's overloads; every list path shares the shape.
      return (await queryApi(() => list(request, { signal }))) as ListBodyOf<P>;
    },
    getNextPageParam: (lastPage) => {
      const pagination = lastPage.pagination as Pagination | undefined;
      if (!pagination) return null;
      if (byCursor) return pagination.hasMore ? (pagination.nextCursor ?? null) : null;
      const { page = 1, totalPages = page } = pagination;
      return page < totalPages ? page + 1 : null;
    },
  });

  const pages = query.data?.pages;
  const rows = useMemo(() => (pages?.flatMap((page) => page.data) ?? []) as ListRow<P>[], [pages]);
  return { ...query, rows };
}

/**
 * Loads every page of a small lookup list (chart of accounts, tax rates, open invoices of one
 * party…) at the API's maximum page size (100). `isComplete` is true once the last page arrived.
 */
export function useAllRows<P extends ListPath>(
  path: P,
  options: {
    filters?: ListFilters<P>;
    enabled?: boolean;
    pathParams?: Record<string, string>;
  } = {},
) {
  const list = useCursorList(path, { limit: 100, ...options });
  const { hasNextPage, isFetchingNextPage, fetchNextPage } = list;
  useEffect(() => {
    if (hasNextPage && !isFetchingNextPage) fetchNextPage();
  }, [hasNextPage, isFetchingNextPage, fetchNextPage]);
  return { ...list, isComplete: !list.isPending && !hasNextPage };
}
