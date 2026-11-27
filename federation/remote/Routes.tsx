"use client";

import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { EmptyState, PageContainer } from "@zerpto/ui";
import type { ExtensionRoutesProps } from "@zerpto/ui/extension";
import { useEffect, useState } from "react";
import { Toaster } from "sonner";
import { IntlProvider } from "use-intl";
import { ExtensionDetail } from "@/components/settings/extension-detail";
import { ExtensionsSettings } from "@/components/settings/extensions-settings";
import { RoleForm } from "@/components/settings/role-form";
import { RolesSettings } from "@/components/settings/roles-settings";
import { TeamSettings } from "@/components/settings/team-settings";
import { UserPermissions } from "@/components/settings/user-permissions";
import { WorkspaceSettings } from "@/components/settings/workspace-settings";
import { setApiFetch } from "@/lib/api/sdk";
import en from "@/messages/en.json";
import id from "@/messages/id.json";
import { matchPage } from "@/router";
import { HostProvider } from "@/shims/host";
import "./styles.css";

const messages = { en, id };

const NOT_FOUND = {
  en: "Page not found",
  id: "Halaman tidak ditemukan",
};

/** One client per mount: the host remounts the extension when the workspace changes. */
function createQueryClient() {
  return new QueryClient({
    defaultOptions: {
      queries: {
        staleTime: 60_000,
        // Network and 5xx errors get one retry; a 4xx answer is final.
        retry: (failureCount, error) => {
          const status = (error as { status?: number } | null)?.status;
          return failureCount < 1 && (status === undefined || status >= 500);
        },
      },
    },
  });
}

/** Follows the host's `dark` class. */
function useDarkMode() {
  const [dark, setDark] = useState(() => document.documentElement.classList.contains("dark"));
  useEffect(() => {
    const root = document.documentElement;
    const observer = new MutationObserver(() => setDark(root.classList.contains("dark")));
    observer.observe(root, { attributes: true, attributeFilter: ["class"] });
    return () => observer.disconnect();
  }, []);
  return dark;
}

function Screen({ path, search }: { path: string; search: string }) {
  const page = matchPage(path);
  switch (page?.name) {
    case "workspace":
      return <WorkspaceSettings />;
    case "team":
      return <TeamSettings />;
    case "member-permissions":
      return <UserPermissions key={page.userId} userId={page.userId} />;
    case "roles":
      return <RolesSettings />;
    case "new-role": {
      const clone = new URLSearchParams(search).get("clone") ?? undefined;
      return <RoleForm key={`new ${clone}`} cloneFrom={clone} />;
    }
    case "role":
      return <RoleForm key={page.roleId} roleId={page.roleId} />;
    case "extensions":
      return <ExtensionsSettings />;
    case "extension":
      return <ExtensionDetail key={page.id} id={page.id} />;
    default:
      return null;
  }
}

export default function Routes({
  path,
  search,
  locale,
  permissions,
  navigate,
  Link,
  apiFetch,
}: ExtensionRoutesProps) {
  setApiFetch(apiFetch);
  const [queryClient] = useState(createQueryClient);
  const dark = useDarkMode();

  return (
    <HostProvider value={{ path, search, permissions, navigate, Link }}>
      <IntlProvider
        locale={locale}
        messages={messages[locale]}
        timeZone={Intl.DateTimeFormat().resolvedOptions().timeZone}
      >
        <QueryClientProvider client={queryClient}>
          {matchPage(path) ? (
            <Screen path={path} search={search} />
          ) : (
            <PageContainer>
              <EmptyState title={NOT_FOUND[locale]} />
            </PageContainer>
          )}
          <Toaster position="top-center" richColors closeButton theme={dark ? "dark" : "light"} />
        </QueryClientProvider>
      </IntlProvider>
    </HostProvider>
  );
}
