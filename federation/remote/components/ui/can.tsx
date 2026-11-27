"use client";

import { useAccess, useCanManage } from "@/lib/permissions";

/**
 * Renders its children only when the user may change things on the current page, from
 * `data.permissions` of `/auth/me` (`<module>.manage`, see `lib/permissions`). Wrap buttons that
 * create, post, cancel, void or otherwise change a record; links to `/new` and `/edit` and `RowMenu`
 * items already hide themselves.
 */
export function Can({ children }: { children: React.ReactNode }) {
  return useCanManage() ? children : null;
}

/**
 * Renders its children only when the user may view the page at `href` (`<module>.read` or
 * `.manage`, see `lib/permissions`). It waits for the permissions, so children that fetch data
 * (dashboard tiles) never mount, and never call the API, for someone who may not see them.
 */
export function CanView({ href, children }: { href: string; children: React.ReactNode }) {
  const access = useAccess();
  return access.ready && access.can("read", href) ? children : null;
}
