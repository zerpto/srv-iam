/**
 * The SDK clients of these screens. Every request goes through the host's `apiFetch`, which adds
 * and renews the access token; this code never sees it.
 */
import {
  BillingApi,
  Configuration,
  RegionsApi,
  RolesAndPermissionsApi,
  TenantsApi,
  UsersApi,
  AuthenticationApi,
} from "@zerpto/sdk";
import type { ExtensionApiFetch } from "@zerpto/ui/extension";
import { ExtensionRegistryApi, ExtensionsApi } from "./extensions-client";

let apiFetch: ExtensionApiFetch | undefined;

/** Called by `Routes` with the host's prop before anything is requested. */
export function setApiFetch(next: ExtensionApiFetch) {
  apiFetch = next;
}

const configuration = new Configuration({
  // The SDK then builds `/v1/...` paths, which is what `apiFetch` takes.
  basePath: "",
  fetchApi: (input, init) => {
    if (!apiFetch) throw new TypeError("The host's apiFetch is not set.");
    return apiFetch(String(input), init);
  },
});

export const sdk = {
  /** Only `getMe`: the host refuses an extension every other `/v1/auth` call. */
  authentication: new AuthenticationApi(configuration),
  tenants: new TenantsApi(configuration),
  users: new UsersApi(configuration),
  roles: new RolesAndPermissionsApi(configuration),
  regions: new RegionsApi(configuration),
  billing: new BillingApi(configuration),
  /** Local until the SDK has it, see `extensions-client.ts`. */
  extensions: new ExtensionsApi(configuration),
  extensionRegistry: new ExtensionRegistryApi(configuration),
};
