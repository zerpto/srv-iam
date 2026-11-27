"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { DocumentDuplicateIcon } from "@heroicons/react/24/outline";
import { useQueryClient } from "@tanstack/react-query";
import { useTranslations } from "next-intl";
import { useRouter } from "next/navigation";
import { useMemo, useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { toast } from "sonner";
import { TextLink } from "@/components/ui/text-link";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Section } from "@/components/ui/detail";
import { PageContainer, PageFooter, PageHeader, QueryError } from "@/components/ui/page";
import { SelectField, type SelectOption } from "@/components/ui/select-field";
import { TextField } from "@/components/ui/text-field";
import { TextAreaField } from "@/components/ui/textarea-field";
import { invalidateResource } from "@/lib/api/invalidate";
import { createRole, updateRole } from "@/lib/api/platform-api";
import { useCanManage } from "@/lib/permissions";
import { useApiErrorMessage } from "@/lib/api/use-error-message";
import { applyActionError } from "@/lib/forms";
import { type Role, useMyPermissions, useRoles } from "@/lib/roles";
import { createRoleSchema, type RoleValues } from "@/lib/validations/platform";
import { PermissionPicker } from "./permission-picker";

const LIST = "/settings/roles";

/**
 * Create a custom role (optionally starting from another role), edit a custom role, or view a
 * system role (read-only, with a way to clone it).
 */
export function RoleForm({ roleId, cloneFrom }: { roleId?: string; cloneFrom?: string }) {
  const t = useTranslations("roles");
  const { roles, isPending, isError, error, refetch } = useRoles();

  if (isPending) {
    return (
      <PageContainer>
        <Section>
          <div aria-busy="true" className="h-96 animate-pulse bg-zinc-100 dark:bg-white/5" />
        </Section>
      </PageContainer>
    );
  }
  if (isError) {
    return (
      <PageContainer>
        <Section>
          <QueryError error={error} onRetry={() => refetch()} />
        </Section>
      </PageContainer>
    );
  }

  const existing = roleId ? roles.find((role) => role.id === roleId) : undefined;
  if (roleId && !existing) {
    return (
      <PageContainer>
        <Section>
          <Section.Content className="py-8 text-center text-sm text-zinc-500 dark:text-zinc-400">
            {t("notFound")}
          </Section.Content>
        </Section>
      </PageContainer>
    );
  }
  const source = existing ?? roles.find((role) => role.id === cloneFrom);
  return <RoleFormBody key={roleId ?? "new"} role={existing} source={source} roles={roles} />;
}

function RoleFormBody({ role, source, roles }: { role?: Role; source?: Role; roles: Role[] }) {
  const t = useTranslations("roles");
  const tc = useTranslations("common");
  const tv = useTranslations("validation");
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();
  const router = useRouter();
  const mine = useMyPermissions();
  const [formError, setFormError] = useState<string | null>(null);
  const schema = useMemo(() => createRoleSchema(tv), [tv]);
  const canManage = useCanManage();
  // System roles are built in; any role is read-only for someone who may not change roles.
  const system = role?.type === "system";
  const readOnly = system || (Boolean(role) && !canManage);
  const {
    register,
    control,
    handleSubmit,
    setError,
    setValue,
    reset,
    formState: { errors, isSubmitting, isDirty },
  } = useForm<RoleValues>({
    resolver: zodResolver(schema),
    defaultValues: {
      name: role ? role.name : source ? `${source.name}-copy` : "",
      description: (role ?? source)?.description ?? "",
      cloneFrom: role ? "" : (source?.id ?? ""),
      permissions: (role ?? source)?.permissions ?? [],
    },
  });

  const cloneOptions: SelectOption<string>[] = useMemo(
    () => [
      { value: "", label: t("startBlank") },
      ...roles.map((r) => ({ value: r.id, label: r.name })),
    ],
    [roles, t],
  );

  async function onSubmit(values: RoleValues) {
    setFormError(null);
    const description = values.description || null;
    const result = role
      ? await updateRole(role.id, {
          name: values.name,
          description,
          permissions: values.permissions,
        })
      : await createRole({ name: values.name, description, permissions: values.permissions });

    if (!result.ok) {
      setFormError(
        applyActionError(result, errorMessage(result), setError, {
          name: "name",
          description: "description",
          permissions: "permissions",
        }),
      );
      return;
    }
    await invalidateResource(queryClient, "/v1/iam/roles");
    toast.success(t(role ? "saved" : "created", { name: result.data.name }));
    if (role) {
      // Editing stays on the page: the saved values become the new baseline, so Save goes idle.
      reset({ ...values, description: values.description });
      return;
    }
    router.push(LIST);
  }

  return (
    <PageContainer>
      <form noValidate onSubmit={handleSubmit(onSubmit)}>
        <PageHeader
          title={
            role ? (readOnly ? role.name : t("editTitle", { name: role.name })) : t("createTitle")
          }
          description={system ? t("systemNote") : undefined}
          actions={
            system ? (
              <TextLink
                href={`${LIST}/new?clone=${encodeURIComponent(role.id)}`}
                variant="secondary"
              >
                <DocumentDuplicateIcon aria-hidden="true" className="size-5" />
                {t("clone")}
              </TextLink>
            ) : undefined
          }
        />
        {formError && <Alert>{formError}</Alert>}
        {role && !readOnly && (
          <Alert tone="info">{t("editNote", { count: role.userCount ?? 0 })}</Alert>
        )}

        <Section.Wrapper>
          <Section>
            <Section.Content className="flex flex-col gap-3">
              <div className="flex items-center gap-2">
                <h2 className="text-sm font-semibold text-zinc-900 dark:text-white">
                  {t("details")}
                </h2>
                {system && <Badge>{t("systemBadge")}</Badge>}
              </div>
              <div className="grid gap-3.5 sm:grid-cols-2">
                <TextField
                  label={t("name")}
                  hint={t("nameHint")}
                  autoFocus={!readOnly}
                  disabled={readOnly}
                  error={errors.name?.message}
                  {...register("name")}
                />
                {!role && (
                  <Controller
                    name="cloneFrom"
                    control={control}
                    render={({ field }) => (
                      <SelectField
                        label={t("startFrom")}
                        value={field.value}
                        onChange={(id) => {
                          field.onChange(id);
                          const from = roles.find((r) => r.id === id);
                          // Starting from a role copies its permissions into the picker below.
                          setValue("permissions", from?.permissions ?? [], { shouldDirty: true });
                        }}
                        options={cloneOptions}
                      />
                    )}
                  />
                )}
              </div>
              <TextAreaField
                label={t("descriptionLabel")}
                rows={2}
                disabled={readOnly}
                error={errors.description?.message}
                {...register("description")}
              />
            </Section.Content>
          </Section>

          <Section>
            <Section.Content className="flex flex-col gap-3">
              <div>
                <h2 className="text-sm font-semibold text-zinc-900 dark:text-white">
                  {t("permissions")}
                </h2>
                <p className="text-sm text-zinc-500 dark:text-zinc-400">
                  {readOnly ? t("permissionsReadOnly") : t("permissionsHint")}
                </p>
              </div>
              <Controller
                name="permissions"
                control={control}
                render={({ field }) => (
                  <PermissionPicker
                    value={field.value}
                    onChange={(next) => field.onChange(next)}
                    readOnly={readOnly}
                    isLocked={(key) => !mine.has(key)}
                  />
                )}
              />
              {errors.permissions?.message && (
                <p role="alert" className="text-danger-700 text-sm">
                  {errors.permissions.message}
                </p>
              )}
            </Section.Content>
          </Section>
        </Section.Wrapper>

        {!readOnly && (
          <PageFooter>
            <TextLink href={LIST} variant="secondary">
              {tc("cancel")}
            </TextLink>
            <Button type="submit" loading={isSubmitting} disabled={Boolean(role) && !isDirty}>
              {isSubmitting ? tc("saving") : role ? tc("save") : t("createSubmit")}
            </Button>
          </PageFooter>
        )}
      </form>
    </PageContainer>
  );
}
