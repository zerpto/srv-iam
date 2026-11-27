import type { EditUserValues, InviteUserValues } from "@/lib/validations/platform";
import type { RoleInput, RoleUpdate, TenantUpdate, UserPermissionsInput } from "@zerpto/sdk";
import { callApi } from "./result";
import { sdk } from "./sdk";

/** PATCH /v1/iam/tenant. Only the fields passed are changed. */
export function updateTenant(body: TenantUpdate) {
  return callApi(() => sdk.tenants.updateTenant({ tenantUpdate: body }));
}

/** POST /v1/iam/users. Sends the invitation email (user status starts as `invited`). */
export function inviteUser(values: InviteUserValues, idempotencyKey: string) {
  return callApi(() =>
    sdk.users.createUser({
      idempotencyKey: idempotencyKey,
      userInput: { email: values.email, roleIds: values.roleIds },
    }),
  );
}

/** PATCH /v1/iam/users/{id}. Replaces the user's roles; the name belongs to the person. */
export function updateUser(id: string, values: EditUserValues) {
  return callApi(() => sdk.users.updateUser({ id, userUpdate: { roleIds: values.roleIds } }));
}

/** DELETE /v1/iam/users/{id}/activate (the user record is kept, status `deactivated`). */
export function deactivateUser(id: string) {
  return callApi(() => sdk.users.deactivateUser({ id }));
}

/** PUT /v1/iam/users/{id}/activate. Restores a deactivated user with their roles. */
export function reactivateUser(id: string) {
  return callApi(() => sdk.users.reactivateUser({ id }));
}

/** DELETE /v1/iam/users/{id}. Cancels an invitation that hasn't been accepted (status `invited`). */
export function revokeUserInvitation(id: string) {
  return callApi(() => sdk.users.revokeUserInvitation({ id }));
}

/** POST /v1/iam/roles. Pass `clone_from_role_id` instead of `permissions` to copy another role. */
export function createRole(body: RoleInput) {
  return callApi(() => sdk.roles.createRole({ roleInput: body }));
}

/** PATCH /v1/iam/roles/{id}. `permissions` replaces the whole set; system roles are rejected. */
export function updateRole(id: string, body: RoleUpdate) {
  return callApi(() => sdk.roles.updateRole({ id, roleUpdate: body }));
}

/** DELETE /v1/iam/roles/{id}. Fails with `role_in_use` while someone holds it. */
export function deleteRole(id: string) {
  return callApi(() => sdk.roles.deleteRole({ id }));
}

/** PUT /v1/iam/users/{id}/permissions. Replaces the user's whole set of direct grants. */
export function setUserPermissions(id: string, grants: UserPermissionsInput["grants"]) {
  return callApi(() => sdk.roles.setUserPermissions({ id, userPermissionsInput: { grants } }));
}

/**
 * POST /v1/iam/extensions/{id}/enable. Also switches on what it depends on: `alsoEnabled` holds the
 * keys of those (from `meta`, which `callApi` would otherwise drop).
 */
export function enableExtension(id: string) {
  return callApi(async () => {
    const { data, meta } = await sdk.extensions.enableExtension({ id });
    return { extension: data, alsoEnabled: meta.alsoEnabled };
  });
}

/** POST /v1/iam/extensions/{id}/disable. 409 for a core extension or one others depend on. */
export function disableExtension(id: string) {
  return callApi(() => sdk.extensions.disableExtension({ id }));
}

/**
 * POST /v1/iam/extensions. The result waits for review.
 *
 * PLATFORM_KEY disabled for now: with the key on, never keep or log it, and don't run this through a
 * TanStack mutation, which would cache it with the variables.
 */
export function registerExtension(
  manifestUrl: string,
  // PLATFORM_KEY disabled for now
  // platformKey: string,
) {
  return callApi(() =>
    sdk.extensionRegistry.registerExtension({
      // PLATFORM_KEY disabled for now
      // platformKey,
      extensionInput: { manifestUrl },
    }),
  );
}
