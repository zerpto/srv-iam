import type { QueryClient } from "@tanstack/react-query";

/**
 * Refreshes a resource everywhere it is cached: its list (`/v1/customers`), its detail queries
 * (`/v1/customers/{id}`) and anything below the path (`/v1/reports` covers every report).
 * Pass several paths to refresh related resources too.
 */
export function invalidateResource(queryClient: QueryClient, ...paths: string[]) {
  return queryClient.invalidateQueries({
    predicate: ({ queryKey }) => {
      const [method, path] = queryKey;
      return (
        method === "get" &&
        typeof path === "string" &&
        paths.some((p) => path === p || path.startsWith(`${p}/`))
      );
    },
  });
}

/** Everything a posted warehouse / order document can change: orders, stock, ledger, reports. */
export function invalidateOperations(queryClient: QueryClient) {
  return invalidateResource(
    queryClient,
    "/v1/purchasing",
    "/v1/sales",
    "/v1/inventory",
    "/v1/ar",
    "/v1/ap",
    "/v1/iam/approvals",
    "/v1/gl/journal-entries",
    "/v1/reports",
  );
}

/** Everything tax and fixed-asset postings can change: their lists, bills, the ledger and reports. */
export function invalidateAccounting(queryClient: QueryClient) {
  return invalidateResource(
    queryClient,
    "/v1/tax",
    "/v1/assets",
    "/v1/ap",
    "/v1/ar",
    "/v1/gl/journal-entries",
    "/v1/reports",
  );
}
