import { sdk } from "./sdk";

/** The paginated lists these screens load, keyed by API path. `useCursorList` calls these. */
export const LISTS = {
  "/v1/iam/users": sdk.users.listUsers.bind(sdk.users),
  "/v1/iam/roles": sdk.roles.listRoles.bind(sdk.roles),
  "/v1/iam/extensions": sdk.extensions.listExtensions.bind(sdk.extensions),
  "/v1/ref/regions": sdk.regions.listRegions.bind(sdk.regions),
} as const;

/** None of these lists is paginated by cursor. */
export const CURSOR_PATHS = new Set<string>();
