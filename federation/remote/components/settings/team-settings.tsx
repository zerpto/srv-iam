"use client";

import type { User } from "@zerpto/sdk";
import { zodResolver } from "@hookform/resolvers/zod";
import {
  EnvelopeIcon,
  KeyIcon,
  ArrowUturnLeftIcon,
  NoSymbolIcon,
  PencilSquareIcon,
  LockClosedIcon,
  UserPlusIcon,
  UsersIcon,
} from "@heroicons/react/24/outline";
import { type QueryClient, useQueryClient } from "@tanstack/react-query";
import { useFormatter, useTranslations } from "next-intl";
import { useMemo, useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { useRouter } from "next/navigation";
import { toast } from "sonner";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Can } from "@/components/ui/can";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { Dialog, DialogActions } from "@/components/ui/dialog";
import { type Column, DataTable } from "@/components/ui/data-table";
import { Section } from "@/components/ui/detail";
import { EmptyState, PageContainer, PageHeader } from "@/components/ui/page";
import { RowMenu } from "@/components/ui/row-menu";
import { CheckboxField } from "@/components/ui/checkbox-field";
import { type SelectOption } from "@/components/ui/select-field";
import { TextField } from "@/components/ui/text-field";
import {
  deactivateUser,
  inviteUser,
  reactivateUser,
  revokeUserInvitation,
  updateUser,
} from "@/lib/api/platform-api";
import { useApiQuery } from "@/lib/api/query";
import { sdk } from "@/lib/api/sdk";
import { useCursorList } from "@/lib/api/use-cursor-list";
import { useRoles } from "@/lib/roles";
import { BILLING_PATH, SUBSCRIPTION_KEY, useSubscription } from "@/components/billing/shared";
import { TextLink } from "@/components/ui/text-link";
import { freeSeats } from "@/lib/billing";
import { useAccess } from "@/lib/permissions";
import { useApiErrorMessage } from "@/lib/api/use-error-message";
import { applyActionError, newIdempotencyKey, shouldRotateIdempotencyKey } from "@/lib/forms";
import { initials } from "@/lib/text";
import {
  createEditUserSchema,
  createInviteUserSchema,
  type EditUserValues,
  type InviteUserValues,
} from "@/lib/validations/platform";

const PAGE_SIZE = 25;
const USERS_KEY = ["get", "/v1/iam/users"];

/** Users, and the seats they occupy (invites and deactivations move them). */
function refreshTeam(queryClient: QueryClient) {
  return Promise.all([
    queryClient.invalidateQueries({ queryKey: USERS_KEY }),
    queryClient.invalidateQueries({ queryKey: SUBSCRIPTION_KEY }),
  ]);
}

const statusTones = {
  active: "success",
  invited: "brand",
  deactivated: "neutral",
} as const;

const statusLabels = {
  active: "statusActive",
  invited: "statusInvited",
  deactivated: "statusDeactivated",
} as const;

/** A user's label: invited people have no name until they accept. */
const displayName = (user: User) => user.name ?? user.email;

/**
 * GET/POST /v1/iam/users, PATCH/DELETE /v1/iam/users/{id}, PUT/DELETE /v1/iam/users/{id}/activate,
 * GET /v1/iam/roles.
 */
export function TeamSettings() {
  const t = useTranslations("team");
  const tc = useTranslations("common");
  const format = useFormatter();
  const me = useApiQuery("/v1/auth/me", {}, () => sdk.authentication.getMe());
  const roles = useRoles();
  const users = useCursorList("/v1/iam/users", { limit: PAGE_SIZE });
  const router = useRouter();
  // Seats: the owner, active users and pending invites each take one. Only people who can see
  // billing load it; for the others the API refuses a full invite with `seat_limit_reached`.
  const access = useAccess();
  const canViewBilling = access.ready && access.can("read", BILLING_PATH);
  const { subscription } = useSubscription({ enabled: canViewBilling });
  const seatsFull = subscription ? freeSeats(subscription) === 0 : false;

  const [inviting, setInviting] = useState(false);
  const [editing, setEditing] = useState<User | null>(null);
  const [deactivating, setDeactivating] = useState<User | null>(null);
  const [revoking, setRevoking] = useState<User | null>(null);

  const roleOptions: SelectOption<string>[] = useMemo(
    () =>
      roles.roles.map((role) => ({
        value: role.id,
        // Role names come from the API in lowercase, e.g. "accountant".
        label: role.name.charAt(0).toUpperCase() + role.name.slice(1),
      })),
    [roles.roles],
  );
  const roleName = (id: string) => roleOptions.find((role) => role.value === id)?.label ?? id;
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();

  async function reactivate(user: User) {
    const result = await reactivateUser(user.id);
    if (!result.ok) {
      toast.error(errorMessage(result));
      return;
    }
    await refreshTeam(queryClient);
    toast.success(t("reactivated", { name: displayName(user) }));
  }

  const columns: Column<User>[] = [
    {
      id: "member",
      header: t("member"),
      cell: (user) => {
        const isMe = user.id === me.data?.user.id;
        return (
          <div className="flex items-center gap-2.5">
            <span className="grid size-6 shrink-0 place-items-center rounded-full bg-zinc-100 text-[10px] font-semibold text-zinc-700 dark:bg-white/10 dark:text-zinc-200">
              {initials(displayName(user))}
            </span>
            <div className="min-w-0">
              <p className="flex items-center gap-2 font-medium text-zinc-900 dark:text-white">
                <span className="max-w-64 truncate">{displayName(user)}</span>
                {isMe && <Badge>{tc("you")}</Badge>}
              </p>
              {/* Below `lg` the email column is hidden; show it under the name. */}
              {user.name && (
                <p className="truncate text-xs text-zinc-500 lg:hidden dark:text-zinc-400">
                  {user.email}
                </p>
              )}
            </div>
          </div>
        );
      },
    },
    {
      id: "email",
      header: tc("email"),
      hideBelow: "lg",
      cell: (user) => (
        <span className="block max-w-72 truncate text-zinc-600 dark:text-zinc-300">
          {user.name ? user.email : t("pendingName")}
        </span>
      ),
    },
    {
      id: "roles",
      header: t("role"),
      hideBelow: "md",
      cell: (user) => (
        <div className="flex gap-1.5">
          {user.roleIds.map((id) => (
            <Badge key={id} className="capitalize">
              {roleName(id)}
            </Badge>
          ))}
        </div>
      ),
    },
    {
      id: "status",
      header: t("status"),
      cell: (user) => <Badge tone={statusTones[user.status]}>{t(statusLabels[user.status])}</Badge>,
    },
    {
      id: "access",
      header: t("access"),
      hideBelow: "lg",
      cell: (user) =>
        user.access === "read_only" ? (
          <Badge>{t("readOnly")}</Badge>
        ) : (
          <span className="text-zinc-500 dark:text-zinc-400">{t("fullAccess")}</span>
        ),
    },
    {
      id: "joined",
      header: t("joined"),
      hideBelow: "lg",
      cell: (user) => (
        <span className="text-zinc-500 dark:text-zinc-400">
          {user.createdAt && format.dateTime(user.createdAt, { dateStyle: "medium" })}
        </span>
      ),
    },
    {
      id: "actions",
      header: tc("actions"),
      srOnly: true,
      align: "right",
      cell: (user) => {
        const isMe = user.id === me.data?.user.id;
        return (
          <RowMenu
            items={[
              // A deactivated user can't change roles until restored (`user_deactivated`).
              {
                label: t("reactivate"),
                icon: ArrowUturnLeftIcon,
                hidden: user.status !== "deactivated",
                onClick: () => reactivate(user),
              },
              {
                label: t("edit"),
                icon: PencilSquareIcon,
                hidden: user.status === "deactivated",
                onClick: () => setEditing(user),
              },
              {
                label: t("permissions"),
                icon: KeyIcon,
                hidden: user.status === "deactivated",
                onClick: () =>
                  router.push(`/settings/team/${encodeURIComponent(user.id)}/permissions`),
              },
              // Invited people are cancelled; accepted ones are deactivated (never yourself).
              {
                label: t("revoke"),
                icon: NoSymbolIcon,
                tone: "danger",
                hidden: user.status !== "invited",
                onClick: () => setRevoking(user),
              },
              {
                label: t("deactivate"),
                icon: NoSymbolIcon,
                tone: "danger",
                hidden: user.status !== "active" || isMe,
                onClick: () => setDeactivating(user),
              },
            ]}
          />
        );
      },
    },
  ];

  const inviteButton = (
    <Can>
      <Button onClick={() => setInviting(true)} disabled={seatsFull}>
        <UserPlusIcon aria-hidden="true" className="size-5" />
        {t("invite")}
        {seatsFull && <LockClosedIcon aria-hidden="true" className="size-4" />}
      </Button>
    </Can>
  );

  return (
    <PageContainer>
      <PageHeader title={t("title")} description={t("description")} actions={inviteButton} />

      <Section.Wrapper>
        {subscription && (
          <div
            role="status"
            className="bleed flex flex-col gap-2 border-y border-zinc-200 bg-white py-2.5 pr-(--gutter) pl-(--row-start) text-sm sm:flex-row sm:items-center sm:justify-between dark:border-white/10 dark:bg-white/5"
          >
            <p className="text-zinc-700 dark:text-zinc-300">
              {t("seatsUsed", {
                used: subscription.occupiedSeats,
                total: subscription.purchasedSeats,
              })}
              {seatsFull &&
                ` ${subscription.status === "free" ? t("seatsFullFree") : t("seatsFull")}`}
            </p>
            {seatsFull && (
              <TextLink href={BILLING_PATH} className="shrink-0 font-semibold">
                {subscription.status === "free" ? t("seatsUpgrade") : t("seatsAdd")}
              </TextLink>
            )}
          </div>
        )}
        <Section>
          <DataTable
            list={users}
            columns={columns}
            rowKey={(user) => user.id}
            empty={
              <EmptyState
                icon={UsersIcon}
                title={t("empty")}
                description={t("emptyBody")}
                action={inviteButton}
              />
            }
          />
        </Section>
      </Section.Wrapper>

      <InviteDialog open={inviting} onClose={() => setInviting(false)} roleOptions={roleOptions} />
      <EditDialog user={editing} onClose={() => setEditing(null)} roleOptions={roleOptions} />
      <DeactivateDialog user={deactivating} onClose={() => setDeactivating(null)} />
      <RevokeDialog user={revoking} onClose={() => setRevoking(null)} />
    </PageContainer>
  );
}

function InviteDialog({
  open,
  onClose,
  roleOptions,
}: {
  open: boolean;
  onClose: () => void;
  roleOptions: SelectOption<string>[];
}) {
  const t = useTranslations();
  const tv = useTranslations("validation");
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();
  const [idempotencyKey, setIdempotencyKey] = useState(newIdempotencyKey);
  const [formError, setFormError] = useState<string | null>(null);
  const schema = useMemo(() => createInviteUserSchema(tv), [tv]);
  const {
    register,
    control,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm({
    resolver: zodResolver(schema),
    defaultValues: { email: "", roleIds: [] as string[] },
  });

  function close() {
    reset();
    setFormError(null);
    onClose();
  }

  async function onSubmit(values: InviteUserValues) {
    setFormError(null);
    const result = await inviteUser(values, idempotencyKey);

    if (!result.ok) {
      if (shouldRotateIdempotencyKey(result)) setIdempotencyKey(newIdempotencyKey());
      const message = errorMessage(result);
      if (result.code === "already_member" || result.code === "user_deactivated") {
        setError("email", { type: "server", message }, { shouldFocus: true });
        return;
      }
      setFormError(
        applyActionError(result, message, setError, {
          email: "email",
          role_ids: "roleIds",
        }),
      );
      return;
    }

    setIdempotencyKey(newIdempotencyKey()); // Next invitation is a new request.
    await refreshTeam(queryClient);
    toast.success(t("team.invited", { email: values.email }));
    close();
  }

  return (
    <Dialog
      open={open}
      onClose={isSubmitting ? () => {} : close}
      icon={UserPlusIcon}
      title={t("team.inviteTitle")}
      description={t("team.inviteBody")}
    >
      <form noValidate onSubmit={handleSubmit(onSubmit)} className="space-y-3.5">
        {formError && <Alert>{formError}</Alert>}
        <TextField
          label={t("fields.email")}
          type="email"
          placeholder={t("fields.emailPlaceholder")}
          icon={EnvelopeIcon}
          error={errors.email?.message}
          {...register("email")}
        />
        <Controller
          name="roleIds"
          control={control}
          render={({ field }) => (
            <RoleChecklist
              value={field.value}
              onChange={field.onChange}
              options={roleOptions}
              error={errors.roleIds?.message}
            />
          )}
        />
        <DialogActions>
          <Button variant="secondary" onClick={close} disabled={isSubmitting}>
            {t("common.cancel")}
          </Button>
          <Button type="submit" loading={isSubmitting}>
            {isSubmitting ? t("team.inviteSubmitting") : t("team.inviteSubmit")}
          </Button>
        </DialogActions>
      </form>
    </Dialog>
  );
}

function EditDialog({
  user,
  onClose,
  roleOptions,
}: {
  user: User | null;
  onClose: () => void;
  roleOptions: SelectOption<string>[];
}) {
  const t = useTranslations();

  return (
    <Dialog
      open={user !== null}
      onClose={onClose}
      icon={PencilSquareIcon}
      title={user ? t("team.editTitle", { name: displayName(user) }) : ""}
      description={t("team.editBody")}
    >
      {/* Keyed so the form re-initializes for each user. */}
      {user && <EditForm key={user.id} user={user} onClose={onClose} roleOptions={roleOptions} />}
    </Dialog>
  );
}

function EditForm({
  user,
  onClose,
  roleOptions,
}: {
  user: User;
  onClose: () => void;
  roleOptions: SelectOption<string>[];
}) {
  const t = useTranslations();
  const tv = useTranslations("validation");
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();
  const [formError, setFormError] = useState<string | null>(null);
  const schema = useMemo(() => createEditUserSchema(tv), [tv]);
  const {
    control,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting, isDirty },
  } = useForm({
    resolver: zodResolver(schema),
    defaultValues: { roleIds: user.roleIds },
  });

  async function onSubmit(values: EditUserValues) {
    setFormError(null);
    const result = await updateUser(user.id, values);

    if (!result.ok) {
      setFormError(
        applyActionError(result, errorMessage(result), setError, { role_ids: "roleIds" }),
      );
      return;
    }

    await refreshTeam(queryClient);
    toast.success(t("team.updated", { name: displayName(user) }));
    onClose();
  }

  return (
    <form noValidate onSubmit={handleSubmit(onSubmit)} className="space-y-3.5">
      {formError && <Alert>{formError}</Alert>}
      <Controller
        name="roleIds"
        control={control}
        render={({ field }) => (
          <RoleChecklist
            value={field.value}
            onChange={field.onChange}
            options={roleOptions}
            error={errors.roleIds?.message}
          />
        )}
      />
      <DialogActions>
        <Button variant="secondary" onClick={onClose} disabled={isSubmitting}>
          {t("common.cancel")}
        </Button>
        <Button type="submit" loading={isSubmitting} disabled={!isDirty}>
          {isSubmitting ? t("common.saving") : t("common.save")}
        </Button>
      </DialogActions>
    </form>
  );
}

function DeactivateDialog({ user, onClose }: { user: User | null; onClose: () => void }) {
  const t = useTranslations("team");
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();

  return (
    <ConfirmDialog
      open={user !== null}
      onClose={onClose}
      title={user ? t("deactivateTitle", { name: displayName(user) }) : ""}
      description={t("deactivateBody")}
      confirmLabel={t("deactivateSubmit")}
      onConfirm={async () => {
        if (!user) return true;
        const result = await deactivateUser(user.id);
        if (!result.ok) {
          toast.error(errorMessage(result));
          return false;
        }
        await refreshTeam(queryClient);
        toast.success(t("deactivated", { name: displayName(user) }));
        return true;
      }}
    />
  );
}

function RevokeDialog({ user, onClose }: { user: User | null; onClose: () => void }) {
  const t = useTranslations("team");
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();

  return (
    <ConfirmDialog
      open={user !== null}
      onClose={onClose}
      title={user ? t("revokeTitle", { name: displayName(user) }) : ""}
      description={t("revokeBody")}
      confirmLabel={t("revokeSubmit")}
      onConfirm={async () => {
        if (!user) return true;
        const result = await revokeUserInvitation(user.id);
        if (!result.ok) {
          toast.error(errorMessage(result));
          return false;
        }
        await refreshTeam(queryClient);
        toast.success(t("revoked", { name: displayName(user) }));
        return true;
      }}
    />
  );
}

/** Pick one or more roles; a member's permissions are the union of their roles. */
function RoleChecklist({
  value,
  onChange,
  options,
  error,
}: {
  value: string[];
  onChange: (value: string[]) => void;
  options: SelectOption<string>[];
  error?: string;
}) {
  const t = useTranslations("team");
  return (
    <fieldset className="space-y-2">
      <legend className="loose:text-sm text-[13px] font-medium text-zinc-900 dark:text-zinc-100">
        {t("roles")}
      </legend>
      <div className="max-h-48 space-y-2 overflow-y-auto rounded-lg p-3 ring-1 ring-zinc-200 ring-inset dark:ring-white/10">
        {options.map((option) => (
          <CheckboxField
            key={option.value}
            checked={value.includes(option.value)}
            onChange={(on) =>
              onChange(on ? [...value, option.value] : value.filter((id) => id !== option.value))
            }
            label={option.label}
          />
        ))}
      </div>
      <p className="text-xs text-zinc-500 dark:text-zinc-400">{t("rolesHint")}</p>
      {error && (
        <p role="alert" className="text-danger-700 dark:text-danger-400 text-sm">
          {error}
        </p>
      )}
    </fieldset>
  );
}
