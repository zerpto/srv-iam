/**
 * What the signed-in user may do here, from the `permissions` the host passes (usable keys only:
 * the plan and a read-only user are already taken into account). For showing and hiding controls;
 * the API enforces access.
 */
import { useMemo } from "react";
import { useHost } from "@/shims/host";

export type AccessLevel = "read" | "manage";

type Rule = { read: string[]; manage: string[] };

const module_ = (name: string): Rule => ({
  read: [`${name}.read`, `${name}.manage`],
  manage: [`${name}.manage`],
});

/** An empty list leaves that level open. */
const AREAS: Record<string, Rule> = {
  "/settings/team": module_("iam.users"),
  "/settings/roles": module_("iam.roles"),
  "/settings/workspace": { read: [], manage: ["iam.tenant.manage"] },
  "/settings/extensions": { read: [], manage: ["iam.extensions.manage"] },
  // Billing is not part of this edition: shown only to someone who holds its permission.
  "/settings/billing": module_("billing.subscription"),
};

const isUnder = (path: string, base: string) => path === base || path.startsWith(`${base}/`);

/** True for a URL that creates or edits a record: `/…/new`, `/…/[id]/edit`, `/…/[id]/permissions`. */
export function isChangePath(path: string) {
  const clean = path.split(/[?#]/)[0].replace(/\/+$/, "");
  return /\/(new|edit|permissions)$/.test(clean);
}

export function useAccess() {
  const { permissions } = useHost();
  return useMemo(() => {
    const held = new Set(permissions);
    return {
      ready: true,
      can: (level: AccessLevel, path: string) => {
        const clean = path.split(/[?#]/)[0];
        const rule = Object.entries(AREAS).find(([prefix]) => isUnder(clean, prefix))?.[1];
        const keys = rule?.[level] ?? [];
        return keys.length === 0 || keys.some((key) => held.has(key));
      },
    };
  }, [permissions]);
}

/** Can the user change things on the current page? */
export function useCanManage() {
  const { path } = useHost();
  return useAccess().can("manage", path);
}
