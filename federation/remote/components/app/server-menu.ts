/** The server menu's group keys as the keys of `shell.groups` in the messages. */
export const GROUP_IDS: Partial<
  Record<string, "sales" | "purchases" | "inventory" | "accounting" | "taxAssets" | "catalog">
> = {
  sales: "sales",
  purchases: "purchases",
  inventory: "inventory",
  accounting: "accounting",
  "tax-assets": "taxAssets",
  catalog: "catalog",
};
