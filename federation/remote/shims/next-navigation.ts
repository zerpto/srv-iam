/** The part of `next/navigation` the screens use, on top of the host's props. */
import { useMemo } from "react";
import { useHost } from "./host";

export function usePathname() {
  return useHost().path;
}

export function useSearchParams() {
  const { search } = useHost();
  return useMemo(() => new URLSearchParams(search), [search]);
}

export function useRouter() {
  const { navigate } = useHost();
  return useMemo(
    () => ({
      push: (href: string) => navigate(href),
      replace: (href: string) => navigate(href, { replace: true }),
      back: () => window.history.back(),
    }),
    [navigate],
  );
}
