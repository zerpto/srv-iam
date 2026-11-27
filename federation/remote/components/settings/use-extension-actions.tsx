"use client";

import { useQueryClient } from "@tanstack/react-query";
import { useFormatter, useTranslations } from "next-intl";
import { useState } from "react";
import { toast } from "sonner";
import { ConfirmDialog } from "@/components/ui/confirm-dialog";
import { invalidateResource } from "@/lib/api/invalidate";
import { disableExtension, enableExtension } from "@/lib/api/platform-api";
import { useApiErrorMessage } from "@/lib/api/use-error-message";
import { enabledDependents, type Extension } from "@/lib/extensions";

/**
 * Enabling and disabling extensions. `all` is the extension list, to name the ones switched on along
 * the way and the ones that block disabling. Render `dialog` once on the page.
 */
export function useExtensionActions(all: Extension[]) {
  const t = useTranslations("extensions");
  const format = useFormatter();
  const errorMessage = useApiErrorMessage();
  const queryClient = useQueryClient();
  const [pendingId, setPendingId] = useState<string | null>(null);
  const [disabling, setDisabling] = useState<Extension | null>(null);

  // The user's permissions (`/auth/me`), the catalogue and the sidebar all follow what is enabled.
  const refresh = () =>
    invalidateResource(
      queryClient,
      "/v1/iam/extensions",
      "/v1/auth/me",
      "/v1/iam/permissions",
      "/v1/iam/menu",
    );

  async function enable(extension: Extension) {
    if (pendingId) return;
    setPendingId(extension.id);
    const result = await enableExtension(extension.id);
    if (!result.ok) {
      toast.error(errorMessage(result));
      if (result.status === 404) await refresh();
    } else {
      const names = result.data.alsoEnabled.map(
        (key) => all.find((other) => other.key === key)?.name ?? key,
      );
      toast.success(t("enabledToast", { name: extension.name }), {
        description: names.length
          ? t("alsoEnabled", { names: format.list(names, { type: "conjunction" }) })
          : undefined,
      });
      await refresh();
    }
    setPendingId(null);
  }

  /**
   * The API answers 409 both for a core extension and for one that others depend on, with no error
   * code to tell them apart, so the extension itself and the list say which it is.
   */
  function conflictMessage(extension: Extension, apiMessage: string) {
    if (extension.core) return t("coreError", { name: extension.name });
    const names = enabledDependents(all, extension.key).map((other) => other.name);
    if (names.length === 0)
      return apiMessage || t("requiredUnknownError", { name: extension.name });
    return t("requiredError", {
      name: extension.name,
      names: format.list(names, { type: "conjunction" }),
      count: names.length,
    });
  }

  async function confirmDisable() {
    if (!disabling) return true;
    const result = await disableExtension(disabling.id);
    if (!result.ok) {
      const conflict = result.status === 409;
      toast.error(conflict ? conflictMessage(disabling, result.message) : errorMessage(result));
      // Retrying can't succeed after these, and what is on screen is out of date.
      if (conflict || result.status === 404) {
        await refresh();
        return true;
      }
      return false;
    }
    await refresh();
    toast.success(t("disabledToast", { name: disabling.name }));
    return true;
  }

  const dialog = (
    <ConfirmDialog
      open={disabling !== null}
      onClose={() => setDisabling(null)}
      title={disabling ? t("disableTitle", { name: disabling.name }) : ""}
      description={t("disableBody")}
      confirmLabel={t("disable")}
      onConfirm={confirmDisable}
    />
  );

  return { enable, askDisable: setDisabling, pendingId, dialog };
}
