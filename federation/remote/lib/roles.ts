import type { Permission } from "@zerpto/sdk";
export type { Role, Permission } from "@zerpto/sdk";
import { useMemo } from "react";
import { useApiQuery } from "@/lib/api/query";
import { sdk } from "@/lib/api/sdk";
import { useAllRows } from "@/lib/api/use-cursor-list";

/** Every role of the workspace (system roles and custom ones), all pages. */
export function useRoles() {
  const list = useAllRows("/v1/iam/roles");
  return { ...list, roles: list.rows };
}

/** The permission catalogue (not paginated), grouped by module. */
export function usePermissionCatalog() {
  const query = useApiQuery("/v1/iam/permissions", {}, () => sdk.roles.listPermissions(), {
    staleTime: 5 * 60_000,
  });
  const permissions = useMemo(() => query.data?.data ?? [], [query.data]);
  const modules = useMemo(() => {
    const byModule = new Map<string, Permission[]>();
    for (const permission of [...permissions].sort((a, b) => a.key.localeCompare(b.key))) {
      byModule.set(permission.module, [...(byModule.get(permission.module) ?? []), permission]);
    }
    return [...byModule.entries()].map(([module, items]) => ({ module, items }));
  }, [permissions]);
  return { ...query, permissions, modules };
}

/** A held permission with `*` in it (`*`, `account.*`, `*.manage`) as a matcher; `*` is any run of characters. */
function wildcard(pattern: string) {
  const source = pattern
    .split("*")
    .map((part) => part.replace(/[.+?^${}()|[\]\\]/g, "\\$&"))
    .join(".*");
  return new RegExp(`^${source}$`);
}

/**
 * What the signed-in user holds; they can only hand out permissions from this set. The owner role
 * comes back as the single wildcard `*`, and others may hold patterns such as `account.*` or
 * `*.manage`, so `has` matches wildcards as well as exact keys.
 */
export function useMyPermissions() {
  const me = useApiQuery("/v1/auth/me", {}, () => sdk.authentication.getMe());
  return useMemo(() => {
    const held = me.data?.permissions ?? [];
    const exact = new Set(held.filter((key) => !key.includes("*")));
    const patterns = held.filter((key) => key.includes("*")).map(wildcard);
    return {
      /** False until `/auth/me` has answered. */
      ready: Boolean(me.data),
      /**
       * The tenant is on the free plan or overdue and this person isn't the owner: every write is
       * refused with 403 `read_only_user`, whatever the permissions say.
       */
      readOnly: me.data?.access === "read_only",
      has: (key: string) => exact.has(key) || patterns.some((pattern) => pattern.test(key)),
    };
  }, [me.data]);
}
