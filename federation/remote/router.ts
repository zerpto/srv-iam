/** Which screen an app path shows. The paths are the ones web.app used for these screens. */
export type Page =
  | { name: "workspace" | "team" | "roles" | "extensions" }
  | { name: "member-permissions"; userId: string }
  | { name: "new-role" }
  | { name: "role"; roleId: string }
  | { name: "extension"; id: string }
  | null;

export function matchPage(path: string): Page {
  const [root, area, id, action, ...rest] = path.split("/").filter(Boolean);
  if (root !== "settings" || rest.length > 0) return null;
  if (!id) {
    return area === "workspace" || area === "team" || area === "roles" || area === "extensions"
      ? { name: area }
      : null;
  }
  if (area === "team") {
    return action === "permissions" ? { name: "member-permissions", userId: id } : null;
  }
  if (action) return null;
  if (area === "roles") return id === "new" ? { name: "new-role" } : { name: "role", roleId: id };
  return area === "extensions" ? { name: "extension", id } : null;
}
