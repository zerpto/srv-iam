"use client";

import type { Tenant, TenantUpdate } from "@zerpto/sdk";
import { zodResolver } from "@hookform/resolvers/zod";
import { LockClosedIcon } from "@heroicons/react/20/solid";
import { useQueryClient } from "@tanstack/react-query";
import { useFormatter, useTranslations } from "next-intl";
import { useMemo, useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { toast } from "sonner";
import { AddressFields } from "@/components/address/address-fields";
import { Alert } from "@/components/ui/alert";
import { Button } from "@/components/ui/button";
import { DetailMeta, Section } from "@/components/ui/detail";
import {
  PageContainer,
  PageFooter,
  PageHeader,
  QueryError,
  SettingsSection,
} from "@/components/ui/page";
import { SelectField } from "@/components/ui/select-field";
import { TextField } from "@/components/ui/text-field";
import { TimezoneField } from "@/components/ui/timezone-field";
import { updateTenant } from "@/lib/api/platform-api";
import { useApiQuery } from "@/lib/api/query";
import { sdk } from "@/lib/api/sdk";
import { useCanManage } from "@/lib/permissions";
import { useApiErrorMessage } from "@/lib/api/use-error-message";
import { addressFromApi, addressToBody } from "@/lib/address";
import { applyActionError } from "@/lib/forms";
import { useCurrencyOptions, useMonthOptions } from "@/lib/use-intl-options";
import {
  createWorkspaceSchema,
  normalizeTaxId,
  type WorkspaceValues,
} from "@/lib/validations/platform";

function toFormValues(tenant: Tenant): WorkspaceValues {
  return {
    name: tenant.name,
    legalName: tenant.legalName ?? "",
    taxId: tenant.taxId ?? "",
    address: addressFromApi(tenant.address),
    timezone: tenant.timezone,
    fiscalYearStartMonth: tenant.fiscalYearStartMonth ?? 1,
  };
}

/** Builds a PATCH body with only the sections the user changed. */
function toUpdate(values: WorkspaceValues, dirty: Partial<Record<keyof WorkspaceValues, unknown>>) {
  const body: TenantUpdate = {};
  if (dirty.name) body.name = values.name;
  if (dirty.legalName) body.legalName = values.legalName;
  if (dirty.taxId) body.taxId = normalizeTaxId(values.taxId) || null;
  if (dirty.address) {
    body.address = addressToBody(values.address);
  }
  if (dirty.timezone) body.timezone = values.timezone;
  if (dirty.fiscalYearStartMonth) body.fiscalYearStartMonth = values.fiscalYearStartMonth;
  return body;
}

/** GET + PATCH /v1/iam/tenant */
export function WorkspaceSettings() {
  const t = useTranslations("workspaceSettings");
  const { data, isPending, isError, error, refetch } = useApiQuery("/v1/iam/tenant", {}, () =>
    sdk.tenants.getTenant(),
  );

  return (
    <PageContainer>
      <PageHeader title={t("title")} description={t("description")} />
      {isPending ? (
        <FormSkeleton />
      ) : isError ? (
        <Section>
          <QueryError error={error} onRetry={() => refetch()} />
        </Section>
      ) : (
        // Remount when the tenant changes (e.g. after switching workspace).
        // Everyone can read the workspace; changing it needs `tenant.manage`.
        <Workspace key={data.id} tenant={data} />
      )}
    </PageContainer>
  );
}

function Workspace({ tenant }: { tenant: Tenant }) {
  const canManage = useCanManage();
  return canManage ? <WorkspaceForm tenant={tenant} /> : <WorkspaceView tenant={tenant} />;
}

/** The workspace as plain text, for people who may view it but not change it. */
function WorkspaceView({ tenant }: { tenant: Tenant }) {
  const t = useTranslations("workspaceSettings");
  const format = useFormatter();
  const monthOptions = useMonthOptions();
  const currencyOptions = useCurrencyOptions();
  const address = tenant.address;
  const addressLines = [
    [address?.line1, address?.line2].filter(Boolean).join(", "),
    [address?.village, address?.subDistrict, address?.district].filter(Boolean).join(", "),
    [address?.province, address?.postalCode].filter(Boolean).join(" "),
    address?.country,
  ].filter(Boolean);
  const currency =
    currencyOptions.find((option) => option.value === tenant.baseCurrency)?.label ??
    tenant.baseCurrency;
  const month = monthOptions.find((option) => option.value === (tenant.fiscalYearStartMonth ?? 1));

  return (
    <Section.Wrapper>
      <Section>
        <Section.Content>
          <SettingsSection title={t("general")} description={t("generalBody")}>
            <dl className="space-y-3">
              <DetailMeta label={t("name")}>{tenant.name}</DetailMeta>
              <DetailMeta label={t("legalName")}>{tenant.legalName || "—"}</DetailMeta>
              <DetailMeta label={t("taxId")}>{tenant.taxId || "—"}</DetailMeta>
            </dl>
            {tenant.createdAt && (
              <p className="text-xs text-zinc-500 dark:text-zinc-400">
                {t("createdOn", {
                  date: format.dateTime(new Date(tenant.createdAt), { dateStyle: "long" }),
                })}
              </p>
            )}
          </SettingsSection>
        </Section.Content>
      </Section>

      <Section>
        <Section.Content>
          <SettingsSection title={t("address")} description={t("addressBody")}>
            <dl>
              <DetailMeta label={t("address")}>
                {addressLines.length > 0
                  ? addressLines.map((line) => (
                      <span key={line} className="block">
                        {line}
                      </span>
                    ))
                  : "—"}
              </DetailMeta>
            </dl>
          </SettingsSection>
        </Section.Content>
      </Section>

      <Section>
        <Section.Content>
          <SettingsSection title={t("regional")} description={t("regionalBody")}>
            <dl className="space-y-3">
              <DetailMeta label={t("baseCurrency")}>{currency}</DetailMeta>
              <DetailMeta label={t("timezone")}>{tenant.timezone}</DetailMeta>
              <DetailMeta label={t("fiscalYear")}>{month?.label ?? "—"}</DetailMeta>
            </dl>
          </SettingsSection>
        </Section.Content>
      </Section>
    </Section.Wrapper>
  );
}

function WorkspaceForm({ tenant }: { tenant: Tenant }) {
  const t = useTranslations("workspaceSettings");
  const tc = useTranslations("common");
  const tv = useTranslations("validation");
  const format = useFormatter();
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();
  const monthOptions = useMonthOptions();
  const currencyOptions = useCurrencyOptions();
  const [formError, setFormError] = useState<string | null>(null);
  const schema = useMemo(() => createWorkspaceSchema(tv), [tv]);
  const {
    register,
    control,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting, isDirty, dirtyFields },
  } = useForm({
    resolver: zodResolver(schema),
    defaultValues: toFormValues(tenant),
  });

  const currencyLabel =
    currencyOptions.find((option) => option.value === tenant.baseCurrency)?.label ??
    tenant.baseCurrency;

  async function onSubmit(values: WorkspaceValues) {
    setFormError(null);
    const result = await updateTenant(toUpdate(values, dirtyFields));

    if (!result.ok) {
      setFormError(
        applyActionError(result, errorMessage(result), setError, {
          name: "name",
          legal_name: "legalName",
          tax_id: "taxId",
          "address.country": "address.country",
          timezone: "timezone",
          fiscal_year_start_month: "fiscalYearStartMonth",
        }),
      );
      return;
    }

    reset(toFormValues(result.data));
    queryClient.setQueryData(["get", "/v1/iam/tenant", {}], result.data);
    // The name also appears in the sidebar switcher.
    await queryClient.invalidateQueries({ queryKey: ["get", "/v1/auth/me"] });
    await queryClient.invalidateQueries({ queryKey: ["get", "/v1/iam/tenants"] });
    toast.success(t("saved"));
  }

  return (
    <form noValidate onSubmit={handleSubmit(onSubmit)}>
      {formError && <Alert>{formError}</Alert>}
      <Section.Wrapper>
        <Section>
          <Section.Content>
            <SettingsSection title={t("general")} description={t("generalBody")}>
              <TextField label={t("name")} error={errors.name?.message} {...register("name")} />
              <TextField
                label={t("legalName")}
                placeholder={t("legalNamePlaceholder")}
                error={errors.legalName?.message}
                {...register("legalName")}
              />
              <TextField
                label={t("taxId")}
                inputMode="numeric"
                placeholder="00.000.000.0-000.000"
                hint={t("taxIdHint")}
                error={errors.taxId?.message}
                {...register("taxId")}
              />
              {tenant.createdAt && (
                <p className="text-xs text-zinc-500 dark:text-zinc-400">
                  {t("createdOn", {
                    date: format.dateTime(new Date(tenant.createdAt), { dateStyle: "long" }),
                  })}
                </p>
              )}
            </SettingsSection>
          </Section.Content>
        </Section>

        <Section>
          <Section.Content>
            <SettingsSection title={t("address")} description={t("addressBody")}>
              <Controller
                name="address"
                control={control}
                render={({ field }) => (
                  <AddressFields
                    value={field.value}
                    onChange={field.onChange}
                    errors={{
                      country: errors.address?.country?.message,
                      province: errors.address?.province?.message,
                      district: errors.address?.district?.message,
                      subDistrict: errors.address?.subDistrict?.message,
                      village: errors.address?.village?.message,
                    }}
                  />
                )}
              />
            </SettingsSection>
          </Section.Content>
        </Section>

        <Section>
          <Section.Content>
            <SettingsSection title={t("regional")} description={t("regionalBody")}>
              <div className="space-y-1.5">
                <p className="text-sm font-medium text-zinc-900 dark:text-zinc-100">
                  {t("baseCurrency")}
                </p>
                <div className="flex items-center gap-2 rounded-lg bg-zinc-50 px-3.5 py-2.5 text-sm text-zinc-700 ring-1 ring-zinc-200 ring-inset dark:bg-white/5 dark:text-zinc-300 dark:ring-white/10">
                  <LockClosedIcon aria-hidden="true" className="size-4 text-zinc-400" />
                  {currencyLabel}
                </div>
                <p className="text-sm text-zinc-500 dark:text-zinc-400">
                  {t("baseCurrencyLocked")}
                </p>
              </div>
              <Controller
                name="timezone"
                control={control}
                render={({ field }) => (
                  <TimezoneField
                    label={t("timezone")}
                    value={field.value}
                    onChange={field.onChange}
                    error={errors.timezone?.message}
                  />
                )}
              />
              <Controller
                name="fiscalYearStartMonth"
                control={control}
                render={({ field }) => (
                  <SelectField
                    label={t("fiscalYear")}
                    value={field.value}
                    onChange={field.onChange}
                    options={monthOptions}
                  />
                )}
              />
            </SettingsSection>
          </Section.Content>
        </Section>
      </Section.Wrapper>

      <PageFooter>
        <Button
          variant="secondary"
          disabled={!isDirty || isSubmitting}
          onClick={() => reset(toFormValues(tenant))}
        >
          {tc("cancel")}
        </Button>
        <Button type="submit" loading={isSubmitting} disabled={!isDirty}>
          {isSubmitting ? tc("saving") : tc("save")}
        </Button>
      </PageFooter>
    </form>
  );
}

function FormSkeleton() {
  return (
    <div aria-busy="true" className="animate-pulse">
      <Section.Wrapper>
        {[0, 1].map((section) => (
          <Section key={section}>
            <Section.Content className="grid gap-3 lg:grid-cols-3">
              <div className="space-y-2">
                <div className="h-4 w-24 rounded bg-zinc-200 dark:bg-white/10" />
                <div className="h-3 w-40 rounded bg-zinc-200 dark:bg-white/10" />
              </div>
              <div className="space-y-3 lg:col-span-2">
                {[0, 1, 2].map((field) => (
                  <div key={field} className="h-10 rounded-lg bg-zinc-200 dark:bg-white/10" />
                ))}
              </div>
            </Section.Content>
          </Section>
        ))}
      </Section.Wrapper>
    </div>
  );
}
