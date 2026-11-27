import type { Extension } from "@/lib/api/extensions-client";
import { useAllRows } from "@/lib/api/use-cursor-list";

export type { Extension } from "@/lib/api/extensions-client";

/** A manifest requirement such as `partner >=1.0`, split into the extension key and version range. */
export function parseRequirement(requirement: string) {
  const [key, ...range] = requirement.trim().split(/\s+/);
  return { key, range: range.join(" ") };
}

/** Every approved extension (`GET /v1/iam/extensions`, all pages), core ones first. */
export function useExtensions() {
  const list = useAllRows("/v1/iam/extensions");
  return { ...list, extensions: list.rows };
}

/** The enabled extensions that require `key`; while there are any, it can't be disabled. */
export function enabledDependents(extensions: Extension[], key: string) {
  return extensions.filter(
    (extension) =>
      extension.enabled &&
      extension.manifest?.requires.services.some((entry) => parseRequirement(entry).key === key),
  );
}
