import { useQuery, type UseQueryOptions } from "@tanstack/react-query";
import { queryApi, type ActionError, type Unwrapped } from "./result";

/**
 * TanStack Query over an SDK call, e.g.
 * `useApiQuery("/v1/auth/me", {}, () => sdk.authentication.getMe())`.
 *
 * The key is `["get", path, request]`: `invalidateResource(queryClient, "/v1/…")` matches on the
 * path, so pass the spec's path template (`/v1/iam/users/{id}`) even though the SDK builds the URL.
 * `data` is the unwrapped record or list; a failure is an `ActionError` (check `status`).
 */
export function useApiQuery<R extends object, T>(
  path: string,
  request: R,
  send: (request: R) => Promise<T>,
  options?: Omit<UseQueryOptions<Unwrapped<T>, ActionError>, "queryKey" | "queryFn">,
) {
  return useQuery<Unwrapped<T>, ActionError>({
    queryKey: ["get", path, request],
    queryFn: () => queryApi(() => send(request)),
    ...options,
  });
}
