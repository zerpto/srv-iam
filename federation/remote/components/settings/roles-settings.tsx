"use client";

import {
  DocumentDuplicateIcon,
  PencilSquareIcon,
  PlusIcon,
  ShieldCheckIcon,
  TrashIcon,
} from "@heroicons/react/24/outline";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useState } from "react";
import { toast } from "sonner";
import { Badge } from "@/components/ui/badge";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { type Column, DataTable } from "@/components/ui/data-table";
import { Section } from "@/components/ui/detail";
import { EmptyState, PageContainer, PageHeader } from "@/components/ui/page";
import { RowMenu } from "@/components/ui/row-menu";
import { SegmentedControl } from "@/components/ui/segmented-control";
import { TextLink } from "@/components/ui/text-link";
import { invalidateResource } from "@/lib/api/invalidate";
import { deleteRole } from "@/lib/api/platform-api";
import { useCursorList } from "@/lib/api/use-cursor-list";
import { useApiErrorMessage } from "@/lib/api/use-error-message";
import type { Role } from "@/lib/roles";
import { useUrlParam } from "@/lib/use-url-state";

const TYPES = ["all", "custom", "system"] as const;

/** GET /v1/iam/roles: system roles (read-only) and the workspace's custom roles. */
export function RolesSettings() {
  const t = useTranslations("roles");
  const tc = useTranslations("common");
  const router = useRouter();
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();
  const [type, setType] = useUrlParam<(typeof TYPES)[number]>("type", "all", TYPES);
  const list = useCursorList("/v1/iam/roles", {
    filters: type === "all" ? undefined : { type },
  });
  const [deleting, setDeleting] = useState<Role | null>(null);

  const columns: Column<Role>[] = [
    {
      id: "name",
      header: t("name"),
      cell: (role) => (
        <div className="min-w-0">
          <TextLink
            href={`/settings/roles/${encodeURIComponent(role.id)}`}
            className="block max-w-64 truncate text-left font-medium capitalize"
          >
            {role.name}
          </TextLink>
          {/* Below `lg` the description column is hidden; show it under the name. */}
          {role.description && (
            <p className="max-w-64 truncate text-xs text-zinc-500 lg:hidden dark:text-zinc-400">
              {role.description}
            </p>
          )}
        </div>
      ),
    },
    {
      id: "description",
      header: t("descriptionLabel"),
      hideBelow: "lg",
      cell: (role) => (
        <span className="block max-w-md truncate text-zinc-600 dark:text-zinc-300">
          {role.description || "—"}
        </span>
      ),
    },
    {
      id: "type",
      header: t("typeColumn"),
      cell: (role) => (
        <Badge tone={role.type === "system" ? "neutral" : "brand"}>
          {role.type === "system" ? t("systemBadge") : t("customBadge")}
        </Badge>
      ),
    },
    {
      id: "permissions",
      header: t("permissions"),
      hideBelow: "md",
      cell: (role) => (
        <span className="text-zinc-600 dark:text-zinc-300">
          {t("permissionCount", { count: role.permissions.length })}
        </span>
      ),
    },
    {
      id: "users",
      header: t("usersColumn"),
      hideBelow: "md",
      cell: (role) => (
        <span className="text-zinc-600 dark:text-zinc-300">
          {t("userCount", { count: role.userCount ?? 0 })}
        </span>
      ),
    },
    {
      id: "actions",
      header: tc("actions"),
      srOnly: true,
      align: "right",
      cell: (role) => (
        <RowMenu
          label={t("actions", { name: role.name })}
          items={[
            {
              label: role.type === "custom" ? tc("edit") : tc("view"),
              icon: PencilSquareIcon,
              view: role.type === "system",
              onClick: () => router.push(`/settings/roles/${encodeURIComponent(role.id)}`),
            },
            {
              label: t("clone"),
              icon: DocumentDuplicateIcon,
              onClick: () =>
                router.push(`/settings/roles/new?clone=${encodeURIComponent(role.id)}`),
            },
            {
              label: t("delete"),
              icon: TrashIcon,
              tone: "danger",
              hidden: role.type !== "custom",
              onClick: () => setDeleting(role),
            },
          ]}
        />
      ),
    },
  ];

  const createButton = (
    <TextLink href="/settings/roles/new" variant="primary">
      <PlusIcon aria-hidden="true" className="size-5" />
      {t("create")}
    </TextLink>
  );

  return (
    <PageContainer>
      <PageHeader title={t("title")} description={t("description")} actions={createButton} />

      <Section.Wrapper>
        <Section>
          <DataTable
            list={list}
            columns={columns}
            rowKey={(role) => role.id}
            filtered={type !== "all"}
            filters={
              <SegmentedControl
                label={t("typeColumn")}
                size="sm"
                value={type}
                onChange={setType}
                options={[
                  { value: "all", label: tc("all") },
                  { value: "custom", label: t("custom") },
                  { value: "system", label: t("system") },
                ]}
              />
            }
            empty={<EmptyState icon={ShieldCheckIcon} title={t("empty")} action={createButton} />}
          />
        </Section>
      </Section.Wrapper>

      <ConfirmDialog
        open={deleting !== null}
        onClose={() => setDeleting(null)}
        title={deleting ? t("deleteTitle", { name: deleting.name }) : ""}
        description={t("deleteBody")}
        confirmLabel={t("delete")}
        onConfirm={async () => {
          if (!deleting) return true;
          const result = await deleteRole(deleting.id);
          if (!result.ok) {
            toast.error(errorMessage(result));
            return false;
          }
          await invalidateResource(queryClient, "/v1/iam/roles");
          toast.success(t("deleted", { name: deleting.name }));
          return true;
        }}
      />
    </PageContainer>
  );
}
