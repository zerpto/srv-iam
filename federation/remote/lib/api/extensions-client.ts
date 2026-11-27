/**
 * TEMPORARY stand-in for the SDK's `ExtensionsApi`: `@zerpto/sdk` was generated before the spec
 * had `/v1/iam/extensions` and `/v1/iam/menu`. Once the SDK is regenerated, delete this file and
 * import `ExtensionsApi`, `Extension`, `Menu`… from `@zerpto/sdk` instead; names follow the
 * generator's (camelCase, one `…Api` class per tag).
 *
 * Both classes extend the SDK runtime's `BaseAPI`. `ExtensionsApi` holds the operations of a
 * signed-in user and gets the app's configuration, auth middleware included. `ExtensionRegistryApi`
 * holds the platform operator's registration, which must get a configuration without that
 * middleware (see `sdk.ts`). Reviewing extensions isn't in the app.
 * PLATFORM_KEY disabled for now: registration doesn't send the operator's key yet.
 */
import { BaseAPI } from "@zerpto/sdk";

export type LocalizedText = { en: string; id: string };

export type MenuItem = {
  key: string;
  group: string | null;
  order: number;
  icon: string;
  label: LocalizedText;
  route: string;
  permission: string | null;
  badge?: string | null;
  createRoute?: string | null;
};

export type MenuGroup = {
  key: string;
  order: number;
  label: LocalizedText;
  items: MenuItem[];
};

export type Menu = { items: MenuItem[]; groups: MenuGroup[] };

export type ManifestPermission = {
  key: string;
  kind: "read" | "write";
  minimumPlan: "free" | "paid";
  description: string;
};

export type ExtensionFrontend = {
  mode: "module-federation";
  /** URL of the remote's `mf-manifest.json`. */
  remote: string;
  /** The exposed module, `./Routes`. */
  expose: string;
  /** Shared libraries and the version range the extension accepts for each. */
  shared: Record<string, string>;
  menu: MenuItem[];
};

export type ExtensionManifest = {
  key: string;
  name: string;
  description: string;
  version: string;
  coreApi: string;
  requires: { services: string[]; grpc: string[]; topics: string[]; permissions: string[] };
  backend: { publishes: string[]; permissions: ManifestPermission[] };
  frontend?: ExtensionFrontend | null;
};

export type Extension = {
  id: string;
  key: string;
  name: string;
  description: string;
  version: string;
  party: string;
  core: boolean;
  status: "pending_review" | "approved" | "rejected";
  /** Whether the signed-in user's tenant uses it. */
  enabled: boolean | null;
  manifest: ExtensionManifest | null;
};

type Pagination = { page: number; perPage: number; totalItems: number; totalPages: number };

type WireMenuItem = Omit<MenuItem, "createRoute"> & { create_route?: string | null };
type WireMenu = {
  items: WireMenuItem[];
  groups: (Omit<MenuGroup, "items"> & { items: WireMenuItem[] })[];
};
type WireManifest = Omit<ExtensionManifest, "coreApi" | "backend" | "frontend"> & {
  core_api: string;
  backend: {
    publishes: string[];
    permissions: (Omit<ManifestPermission, "minimumPlan"> & { minimum_plan: "free" | "paid" })[];
  };
  frontend?: (Omit<ExtensionFrontend, "menu"> & { menu: WireMenuItem[] }) | null;
};
type WireExtension = Omit<Extension, "manifest"> & { manifest: WireManifest | null };
type WirePagination = { page: number; per_page: number; total_items: number; total_pages: number };

function menuItem({ create_route, ...item }: WireMenuItem): MenuItem {
  return { ...item, createRoute: create_route };
}

function extension({ manifest, ...rest }: WireExtension): Extension {
  if (!manifest) return { ...rest, manifest: null };
  const { core_api, backend, frontend, ...other } = manifest;
  return {
    ...rest,
    manifest: {
      ...other,
      coreApi: core_api,
      backend: {
        publishes: backend.publishes,
        permissions: backend.permissions.map(({ minimum_plan, ...permission }) => ({
          ...permission,
          minimumPlan: minimum_plan,
        })),
      },
      frontend: frontend && { ...frontend, menu: frontend.menu.map(menuItem) },
    },
  };
}

export class ExtensionsApi extends BaseAPI {
  private async send<T>(path: string, init?: RequestInit, method: "GET" | "POST" = "GET") {
    const response = await this.request({ path, method, headers: {} }, init);
    return (await response.json()) as T;
  }

  /** GET /v1/iam/extensions */
  async listExtensions(
    { page, perPage }: { page?: number; perPage?: number } = {},
    init?: RequestInit,
  ): Promise<{ data: Extension[]; meta: { pagination: Pagination } }> {
    const response = await this.request(
      {
        path: "/v1/iam/extensions",
        method: "GET",
        headers: {},
        query: { ...(page && { page }), ...(perPage && { per_page: perPage }) },
      },
      init,
    );
    const body = (await response.json()) as {
      data: WireExtension[];
      meta: { pagination: WirePagination };
    };
    const { per_page, total_items, total_pages, page: current } = body.meta.pagination;
    return {
      data: body.data.map(extension),
      meta: {
        pagination: {
          page: current,
          perPage: per_page,
          totalItems: total_items,
          totalPages: total_pages,
        },
      },
    };
  }

  /** GET /v1/iam/extensions/{id} */
  async getExtension({ id }: { id: string }) {
    const body = await this.send<{ data: WireExtension; meta: object }>(
      `/v1/iam/extensions/${encodeURIComponent(id)}`,
    );
    return { data: extension(body.data), meta: body.meta };
  }

  /** POST /v1/iam/extensions/{id}/enable */
  async enableExtension({ id }: { id: string }) {
    const body = await this.send<{ data: WireExtension; meta: { also_enabled: string[] } }>(
      `/v1/iam/extensions/${encodeURIComponent(id)}/enable`,
      undefined,
      "POST",
    );
    return { data: extension(body.data), meta: { alsoEnabled: body.meta.also_enabled } };
  }

  /** POST /v1/iam/extensions/{id}/disable */
  async disableExtension({ id }: { id: string }) {
    const body = await this.send<{ data: WireExtension; meta: object }>(
      `/v1/iam/extensions/${encodeURIComponent(id)}/disable`,
      undefined,
      "POST",
    );
    return { data: extension(body.data), meta: body.meta };
  }

  /** GET /v1/iam/menu */
  async getMenu() {
    const body = await this.send<{ data: WireMenu; meta: object }>("/v1/iam/menu");
    return {
      data: {
        items: body.data.items.map(menuItem),
        groups: body.data.groups.map((group) => ({ ...group, items: group.items.map(menuItem) })),
      } satisfies Menu,
      meta: body.meta,
    };
  }
}

export class ExtensionRegistryApi extends BaseAPI {
  /**
   * POST /v1/iam/extensions. A redirect fails the call.
   *
   * PLATFORM_KEY disabled for now: the platform operator's key goes into this one request's header only. It
   * isn't stored, and failing on a redirect keeps it from being passed on.
   */
  async registerExtension({
    // PLATFORM_KEY disabled for now
    // platformKey,
    extensionInput,
  }: {
    // PLATFORM_KEY disabled for now
    // platformKey: string;
    extensionInput: { manifestUrl: string };
  }) {
    const response = await this.request(
      {
        path: "/v1/iam/extensions",
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          // PLATFORM_KEY disabled for now
          // "X-Platform-Key": platformKey,
        },
        body: { manifest_url: extensionInput.manifestUrl },
      },
      { redirect: "error" },
    );
    const body = (await response.json()) as { data: WireExtension; meta: object };
    return { data: extension(body.data), meta: body.meta };
  }
}
