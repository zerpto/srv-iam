"use client";

import { ArrowLeftIcon } from "@heroicons/react/20/solid";
import clsx from "clsx";
import { useLocale, useTranslations } from "next-intl";
import { GROUP_IDS } from "@/components/app/server-menu";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Can } from "@/components/ui/can";
import { tableStyles } from "@/components/ui/data-table";
import { DetailMeta, detailStyles, Section } from "@/components/ui/detail";
import { PageContainer, PageHeader, QueryError } from "@/components/ui/page";
import { TextLink } from "@/components/ui/text-link";
import type { ExtensionManifest } from "@/lib/api/extensions-client";
import { useApiQuery } from "@/lib/api/query";
import { sdk } from "@/lib/api/sdk";
import {
  enabledDependents,
  type Extension,
  parseRequirement,
  useExtensions,
} from "@/lib/extensions";
import type { Locale } from "@/i18n/config";
import { ExtensionStatusBadge } from "./extension-status-badge";
import { useExtensionActions } from "./use-extension-actions";

const LIST = "/settings/extensions";

/** GET /v1/iam/extensions/{id}: one extension, its manifest, and the enable / disable action. */
export function ExtensionDetail({ id }: { id: string }) {
  const t = useTranslations("extensions");
  const query = useApiQuery("/v1/iam/extensions/{id}", { id }, (request) =>
    sdk.extensions.getExtension(request),
  );
  // Names the dependencies and dependents; the page works without it.
  const { extensions } = useExtensions();
  const actions = useExtensionActions(extensions);

  if (query.isPending) {
    return (
      <PageContainer>
        <BackToList />
        <Section>
          <div aria-busy="true" className="h-96 animate-pulse bg-zinc-100 dark:bg-white/5" />
        </Section>
      </PageContainer>
    );
  }
  if (query.isError) {
    return (
      <PageContainer>
        <BackToList />
        <Section>
          {query.error.status === 404 ? (
            <Section.Content className="py-8 text-center text-sm text-zinc-500 dark:text-zinc-400">
              {t("notFound")}
            </Section.Content>
          ) : (
            <QueryError error={query.error} onRetry={() => query.refetch()} />
          )}
        </Section>
      </PageContainer>
    );
  }

  const extension = query.data;
  const dependents = enabledDependents(extensions, extension.key);

  return (
    <PageContainer>
      <BackToList />
      <PageHeader
        title={extension.name}
        description={extension.description}
        actions={
          extension.core ? undefined : (
            <Can>
              {extension.enabled ? (
                <Button variant="secondary" onClick={() => actions.askDisable(extension)}>
                  {t("disable")}
                </Button>
              ) : (
                <Button
                  loading={actions.pendingId === extension.id}
                  onClick={() => void actions.enable(extension)}
                >
                  {t("enable")}
                </Button>
              )}
            </Can>
          )
        }
      />

      <Section.Wrapper>
        <Section>
          <dl className={clsx(detailStyles.grid, "grid-cols-2 sm:grid-cols-4")}>
            <DetailMeta label={t("status")}>
              <ExtensionStatusBadge extension={extension} />
            </DetailMeta>
            <DetailMeta label={t("type")}>{extension.core ? t("core") : t("optional")}</DetailMeta>
            <DetailMeta label={t("version")}>{extension.version}</DetailMeta>
            <DetailMeta label={t("key")}>
              <span className="font-mono break-all">{extension.key}</span>
            </DetailMeta>
            {extension.manifest && (
              <DetailMeta label={t("coreApi")}>
                <span className="font-mono">{extension.manifest.coreApi}</span>
              </DetailMeta>
            )}
            {!extension.core && dependents.length > 0 && (
              <DetailMeta label={t("requiredBy")} className="col-span-2 sm:col-span-3">
                <span className="flex flex-wrap gap-x-3">
                  {dependents.map((other) => (
                    <ExtensionLink key={other.id} extension={other} />
                  ))}
                </span>
              </DetailMeta>
            )}
          </dl>
          {extension.core && (
            <p className="pr-(--gutter) pb-3 pl-(--row-start) text-sm text-zinc-600 dark:text-zinc-400">
              {t("coreNote")}
            </p>
          )}
        </Section>

        {extension.manifest ? (
          <Manifest manifest={extension.manifest} all={extensions} />
        ) : (
          <Section>
            <Section.Content className="text-sm text-zinc-500 dark:text-zinc-400">
              {t("noManifest")}
            </Section.Content>
          </Section>
        )}
      </Section.Wrapper>

      {actions.dialog}
    </PageContainer>
  );
}

/** What the approved manifest declares: dependencies, permissions and menu entries. */
function Manifest({ manifest, all }: { manifest: ExtensionManifest; all: Extension[] }) {
  const t = useTranslations("extensions");
  const ts = useTranslations("shell");
  const locale = useLocale() as Locale;
  const menu = manifest.frontend?.menu ?? [];
  const groupName = (key: string | null) => {
    const group = key ? GROUP_IDS[key] : undefined;
    return group ? ts(`groups.${group}`) : (key ?? "—");
  };

  return (
    <>
      <Section title={t("dependencies")} subtitle={t("dependenciesHint")}>
        {manifest.requires.services.length === 0 ? (
          <Empty>{t("noDependencies")}</Empty>
        ) : (
          <ul className="divide-y divide-zinc-200 dark:divide-white/10">
            {manifest.requires.services.map((requirement) => {
              const { key, range } = parseRequirement(requirement);
              const found = all.find((extension) => extension.key === key);
              return (
                <li
                  key={requirement}
                  className={clsx(detailStyles.row, "flex flex-wrap items-center gap-3 text-sm")}
                >
                  {found ? (
                    <ExtensionLink extension={found} />
                  ) : (
                    <span className="font-medium text-zinc-900 dark:text-white">{key}</span>
                  )}
                  {range && (
                    <span className="font-mono text-xs text-zinc-500 dark:text-zinc-400">
                      {range}
                    </span>
                  )}
                  {found && <ExtensionStatusBadge extension={found} />}
                </li>
              );
            })}
          </ul>
        )}
      </Section>

      {manifest.requires.permissions.length > 0 && (
        <Section title={t("requiredPermissions")} subtitle={t("requiredPermissionsHint")}>
          <ul className="flex flex-wrap gap-2 pr-(--gutter) pb-3 pl-(--row-start)">
            {manifest.requires.permissions.map((key) => (
              <li key={key}>
                <Badge className="font-mono">{key}</Badge>
              </li>
            ))}
          </ul>
        </Section>
      )}

      <Section title={t("permissions")} subtitle={t("permissionsHint")}>
        {manifest.backend.permissions.length === 0 ? (
          <Empty>{t("noPermissions")}</Empty>
        ) : (
          <div className="overflow-x-auto">
            <table className={tableStyles.table}>
              <thead className={tableStyles.thead}>
                <tr>
                  <th scope="col" className={tableStyles.th}>
                    {t("permission")}
                  </th>
                  <th scope="col" className={clsx(tableStyles.th, "hidden lg:table-cell")}>
                    {t("descriptionColumn")}
                  </th>
                  <th scope="col" className={tableStyles.th}>
                    {t("access")}
                  </th>
                  <th scope="col" className={tableStyles.th}>
                    {t("plan")}
                  </th>
                </tr>
              </thead>
              <tbody className={tableStyles.tbody}>
                {manifest.backend.permissions.map((permission) => (
                  <tr key={permission.key}>
                    <td className={clsx(tableStyles.td, "py-2")}>
                      <span className="font-mono">{permission.key}</span>
                      <p className="max-w-56 truncate text-xs text-zinc-500 lg:hidden dark:text-zinc-400">
                        {permission.description}
                      </p>
                    </td>
                    <td className={clsx(tableStyles.td, "hidden lg:table-cell")}>
                      <span className="block max-w-md truncate" title={permission.description}>
                        {permission.description}
                      </span>
                    </td>
                    <td className={tableStyles.td}>
                      {permission.kind === "write" ? t("write") : t("read")}
                    </td>
                    <td className={tableStyles.td}>
                      <Badge tone={permission.minimumPlan === "paid" ? "brand" : "neutral"}>
                        {permission.minimumPlan === "paid" ? t("paid") : t("free")}
                      </Badge>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Section>

      <Section title={t("menu")} subtitle={t("menuHint")}>
        {menu.length === 0 ? (
          <Empty>{t("noMenu")}</Empty>
        ) : (
          <div className="overflow-x-auto">
            <table className={tableStyles.table}>
              <thead className={tableStyles.thead}>
                <tr>
                  <th scope="col" className={tableStyles.th}>
                    {t("menuEntry")}
                  </th>
                  <th scope="col" className={tableStyles.th}>
                    {t("menuGroup")}
                  </th>
                  <th scope="col" className={clsx(tableStyles.th, "hidden lg:table-cell")}>
                    {t("route")}
                  </th>
                  <th scope="col" className={clsx(tableStyles.th, "hidden lg:table-cell")}>
                    {t("permission")}
                  </th>
                </tr>
              </thead>
              <tbody className={tableStyles.tbody}>
                {menu.map((item) => (
                  <tr key={item.key}>
                    <td className={clsx(tableStyles.td, "py-2")}>
                      <span className="font-medium">{item.label[locale]}</span>
                      <p className="font-mono text-xs text-zinc-500 lg:hidden dark:text-zinc-400">
                        {item.route}
                      </p>
                    </td>
                    <td className={tableStyles.td}>{groupName(item.group)}</td>
                    <td className={clsx(tableStyles.td, "hidden font-mono lg:table-cell")}>
                      {item.route}
                    </td>
                    <td className={clsx(tableStyles.td, "hidden font-mono lg:table-cell")}>
                      {item.permission ?? "—"}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}
      </Section>
    </>
  );
}

function Empty({ children }: { children: React.ReactNode }) {
  return (
    <Section.Content className="pt-0 text-sm text-zinc-500 dark:text-zinc-400">
      {children}
    </Section.Content>
  );
}

function ExtensionLink({ extension }: { extension: Extension }) {
  return (
    <TextLink href={`${LIST}/${encodeURIComponent(extension.id)}`} className="font-medium">
      {extension.name}
    </TextLink>
  );
}

function BackToList() {
  const t = useTranslations("extensions");
  return (
    <TextLink
      href={LIST}
      className="mt-3 inline-flex items-center gap-1.5 text-sm text-zinc-600 dark:text-zinc-400"
    >
      <ArrowLeftIcon aria-hidden="true" className="size-4" />
      {t("title")}
    </TextLink>
  );
}
