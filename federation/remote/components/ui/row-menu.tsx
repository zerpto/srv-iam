"use client";

import { RowMenu as BaseRowMenu, type RowMenuItem as BaseRowMenuItem } from "@zerpto/ui";
import { useCanManage } from "@/lib/permissions";

export type RowMenuItem = BaseRowMenuItem & {
  /** Read-only action (view, open, download): shown to people who may not change this area. */
  view?: boolean;
};

/**
 * "…" actions menu for a table row or card header (the shared one from `@zerpto/ui`, limited to
 * what the user may do here). Renders nothing when every item is hidden.
 */
export function RowMenu({ items, label }: { items: RowMenuItem[]; label?: string }) {
  const canManage = useCanManage();
  return <BaseRowMenu items={items.filter((item) => item.view || canManage)} label={label} />;
}
