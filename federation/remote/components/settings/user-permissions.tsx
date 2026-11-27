"use client";

import { toDateInput } from "@/lib/format";
import { PlusIcon, TrashIcon } from "@heroicons/react/24/outline";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useMemo, useState } from "react";
import { toast } from "sonner";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { ComboboxField } from "@/components/ui/combobox-field";
import { Section, detailStyles } from "@/components/ui/detail";
import { PageContainer, PageFooter, PageHeader, QueryError } from "@/components/ui/page";
import { TextField } from "@/components/ui/text-field";
import { invalidateResource } from "@/lib/api/invalidate";
import { setUserPermissions } from "@/lib/api/platform-api";
import { useApiQuery } from "@/lib/api/query";
import { sdk } from "@/lib/api/sdk";
import { useApiErrorMessage } from "@/lib/api/use-error-message";
import { useMyPermissions, usePermissionCatalog, useRoles } from "@/lib/roles";

const LIST = "/settings/team";

type Grant = { permission: string; expiresAt: string };

/**
 * One member's access: their roles, the permissions those give, and direct grants on top
 * (optionally expiring). GET/PUT /v1/iam/users/{id}/permissions.
 */
export function UserPermissions({ userId }: { userId: string }) {
  const request = { id: userId };
  const user = useApiQuery("/v1/iam/users/{id}", request, (r) => sdk.users.getUser(r));
  const access = useApiQuery("/v1/iam/users/{id}/permissions", request, (r) =>
    sdk.roles.getUserPermissions(r),
  );

  const failed = user.isError ? user : access.isError ? access : null;
  if (failed) {
    return (
      <PageContainer>
        <Section>
          <QueryError error={failed.error} onRetry={() => failed.refetch()} />
        </Section>
      </PageContainer>
    );
  }
  if (!user.data || !access.data) {
    return (
      <PageContainer>
        <Section>
          <div aria-busy="true" className="h-96 animate-pulse bg-zinc-100 dark:bg-white/5" />
        </Section>
      </PageContainer>
    );
  }
  return <Body key={access.dataUpdatedAt} userId={userId} user={user.data} access={access.data} />;
}

function Body({
  userId,
  user,
  access,
}: {
  userId: string;
  user: { name?: string | null; email: string; roleIds: string[] };
  access: { direct: { permission: string; expiresAt?: Date | null }[]; effective: string[] };
}) {
  const t = useTranslations("userPermissions");
  const tc = useTranslations("common");
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();
  const router = useRouter();
  const catalog = usePermissionCatalog();
  const { roles } = useRoles();
  const mine = useMyPermissions();
  const [grants, setGrants] = useState<Grant[]>(() =>
    access.direct.map((g) => ({
      permission: g.permission,
      expiresAt: g.expiresAt ? toDateInput(g.expiresAt) : "",
    })),
  );
  const [adding, setAdding] = useState("");
  const [saving, setSaving] = useState(false);
  const [formError, setFormError] = useState<string | null>(null);

  const descriptions = useMemo(
    () => new Map(catalog.permissions.map((p) => [p.key, p.description])),
    [catalog.permissions],
  );
  const roleNames = roles.filter((r) => user.roleIds.includes(r.id)).map((r) => r.name);
  const directKeys = new Set(grants.map((g) => g.permission));
  const fromRoles = access.effective.filter(
    (key) => !access.direct.some((g) => g.permission === key),
  );
  const options = catalog.permissions
    .filter((p) => !directKeys.has(p.key) && mine.has(p.key))
    .map((p) => ({ value: p.key, label: p.key, description: p.description }));
  const dirty =
    JSON.stringify(grants) !==
    JSON.stringify(
      access.direct.map((g) => ({
        permission: g.permission,
        expiresAt: g.expiresAt ? toDateInput(g.expiresAt) : "",
      })),
    );

  async function save() {
    setSaving(true);
    setFormError(null);
    const result = await setUserPermissions(
      userId,
      grants.map((g) => ({
        permission: g.permission,
        expiresAt: g.expiresAt ? new Date(`${g.expiresAt}T23:59:59`) : null,
      })),
    );
    setSaving(false);
    if (!result.ok) {
      setFormError(errorMessage(result));
      return;
    }
    await invalidateResource(queryClient, "/v1/iam/users");
    toast.success(t("saved", { name: user.name ?? user.email }));
    router.push(LIST);
  }

  return (
    <PageContainer>
      <PageHeader
        title={t("title", { name: user.name ?? user.email })}
        description={t("description", { email: user.email })}
      />
      {formError && <Alert>{formError}</Alert>}

      <Section.Wrapper>
        <Section>
          <Section.Content className="space-y-3">
            <h2 className="text-sm font-semibold text-zinc-900 dark:text-white">
              {t("fromRoles")}
            </h2>
            <p className="flex flex-wrap items-center gap-2 text-sm text-zinc-500 dark:text-zinc-400">
              {t("rolesLine")}
              {roleNames.map((name) => (
                <Badge key={name} tone="brand">
                  {name}
                </Badge>
              ))}
            </p>
            <ul className="flex flex-wrap gap-2">
              {fromRoles.sort().map((key) => (
                <li
                  key={key}
                  title={descriptions.get(key)}
                  className="rounded-md bg-zinc-50 px-2 py-1 font-mono text-xs text-zinc-700 ring-1 ring-zinc-200 dark:bg-white/10 dark:text-zinc-300 dark:ring-white/10"
                >
                  {key}
                </li>
              ))}
            </ul>
          </Section.Content>
        </Section>

        <Section>
          <Section.Content>
            <h2 className="text-sm font-semibold text-zinc-900 dark:text-white">{t("direct")}</h2>
            <p className="text-sm text-zinc-500 dark:text-zinc-400">{t("directHint")}</p>
          </Section.Content>

          {grants.length === 0 ? (
            <p
              className={`${detailStyles.block} border-t border-zinc-200 text-center text-sm text-zinc-500 dark:border-white/10 dark:text-zinc-400`}
            >
              {t("noDirect")}
            </p>
          ) : (
            <ul className="divide-y divide-zinc-200 border-t border-zinc-200 dark:divide-white/10 dark:border-white/10">
              {grants.map((grant) => (
                <li
                  key={grant.permission}
                  className={`${detailStyles.row} flex flex-wrap items-center gap-3`}
                >
                  <div className="min-w-0 flex-1">
                    <p className="truncate font-mono text-xs text-zinc-900 dark:text-white">
                      {grant.permission}
                    </p>
                    <p className="truncate text-xs text-zinc-500 dark:text-zinc-400">
                      {descriptions.get(grant.permission)}
                    </p>
                  </div>
                  <div className="w-40">
                    <TextField
                      compact
                      label={t("expires")}
                      type="date"
                      value={grant.expiresAt}
                      min={new Date().toISOString().slice(0, 10)}
                      onChange={(event) =>
                        setGrants((rows) =>
                          rows.map((r) =>
                            r.permission === grant.permission
                              ? { ...r, expiresAt: event.target.value }
                              : r,
                          ),
                        )
                      }
                    />
                  </div>
                  <button
                    type="button"
                    aria-label={t("remove", { permission: grant.permission })}
                    onClick={() =>
                      setGrants((rows) => rows.filter((r) => r.permission !== grant.permission))
                    }
                    className="focus-visible:outline-brand-600 rounded-lg p-1.5 text-zinc-400 hover:bg-zinc-100 hover:text-zinc-700 focus-visible:outline-2 dark:hover:bg-white/10"
                  >
                    <TrashIcon aria-hidden="true" className="size-4" />
                  </button>
                </li>
              ))}
            </ul>
          )}

          <Section.Content className="space-y-3 border-t border-zinc-200 dark:border-white/10">
            <p className="text-xs text-zinc-500 dark:text-zinc-400">{t("expiresHint")}</p>
            <div className="flex items-end gap-3">
              <div className="min-w-0 flex-1">
                <ComboboxField
                  label={t("add")}
                  value={adding}
                  onChange={(key) => {
                    if (!key) return;
                    setGrants((rows) => [...rows, { permission: key, expiresAt: "" }]);
                    setAdding("");
                  }}
                  options={options}
                  placeholder={t("addPlaceholder")}
                  emptyText={t("noMatch")}
                />
              </div>
              <PlusIcon aria-hidden="true" className="mb-2.5 size-5 text-zinc-400" />
            </div>
          </Section.Content>
        </Section>
      </Section.Wrapper>

      <PageFooter>
        {dirty && <span className="text-sm text-zinc-500 dark:text-zinc-400">{t("unsaved")}</span>}
        <Button variant="secondary" onClick={() => router.push(LIST)} disabled={saving}>
          {tc("cancel")}
        </Button>
        <Button onClick={save} loading={saving} disabled={!dirty}>
          {saving ? tc("saving") : tc("save")}
        </Button>
      </PageFooter>
    </PageContainer>
  );
}
