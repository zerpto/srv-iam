"use client";

import { zodResolver } from "@hookform/resolvers/zod";
import { CheckCircleIcon, PuzzlePieceIcon } from "@heroicons/react/24/outline";
import { useTranslations } from "next-intl";
import { useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { Alert } from "@/components/ui/alert";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { DetailMeta } from "@/components/ui/detail";
import { Dialog, DialogActions } from "@/components/ui/dialog";
// PLATFORM_KEY disabled for now
// import { PasswordField } from "@/components/ui/password-field";
import { TextField } from "@/components/ui/text-field";
import { registerExtension } from "@/lib/api/platform-api";
import { useApiErrorMessage } from "@/lib/api/use-error-message";
import type { Extension } from "@/lib/extensions";
import { applyActionError } from "@/lib/forms";
import {
  createRegisterExtensionSchema,
  type RegisterExtensionValues,
} from "@/lib/validations/platform";

const EMPTY: RegisterExtensionValues = {
  manifestUrl: "",
  // PLATFORM_KEY disabled for now
  // platformKey: "",
};

/**
 * POST /v1/iam/extensions: registers an extension from its manifest address.
 *
 * PLATFORM_KEY disabled for now: with the key on, this is for the platform operator,
 * and the key lives only in this form's state, which is cleared on success and on closing.
 */
export function RegisterExtensionDialog({ open, onClose }: { open: boolean; onClose: () => void }) {
  const t = useTranslations("extensions.register");
  const tx = useTranslations("extensions");
  const tc = useTranslations("common");
  const tv = useTranslations("validation");
  const errorMessage = useApiErrorMessage();
  const [formError, setFormError] = useState<string | null>(null);
  const [registered, setRegistered] = useState<Extension | null>(null);
  const schema = useMemo(() => createRegisterExtensionSchema(tv), [tv]);
  const {
    register,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<RegisterExtensionValues>({ resolver: zodResolver(schema), defaultValues: EMPTY });

  function close() {
    // The request can't be taken back, so stay until its answer is shown.
    if (isSubmitting) return;
    reset(EMPTY);
    setFormError(null);
    setRegistered(null);
    onClose();
  }

  async function onSubmit({ manifestUrl }: RegisterExtensionValues) {
    setFormError(null);
    // PLATFORM_KEY disabled for now: take `platformKey` from the values too and pass it on.
    // const result = await registerExtension(manifestUrl, platformKey);
    const result = await registerExtension(manifestUrl);

    if (result.ok) {
      reset(EMPTY);
      setRegistered(result.data);
      return;
    }
    // PLATFORM_KEY disabled for now: with the key on, a 401 is about the key. Until
    // then it shows below like any other error, with the API's message.
    // if (result.status === 401) {
    //   setError("platformKey", { type: "server", message: t("wrongKey") }, { shouldFocus: true });
    //   return;
    // }
    // The browser doesn't say whether it was the connection or the gateway's CORS rules.
    if (result.code === "network_error") {
      // PLATFORM_KEY disabled for now
      // setFormError(t("networkError"));
      setFormError(t("networkErrorNoKey"));
      return;
    }
    setFormError(
      applyActionError(result, errorMessage(result), setError, { manifest_url: "manifestUrl" }),
    );
  }

  return (
    <Dialog
      open={open}
      onClose={close}
      title={registered ? t("doneTitle") : t("title")}
      // PLATFORM_KEY disabled for now
      // description={registered ? t("doneBody") : t("description")}
      description={registered ? t("doneBody") : t("descriptionNoKey")}
      icon={registered ? CheckCircleIcon : PuzzlePieceIcon}
    >
      {registered ? (
        <>
          <dl className="grid grid-cols-2 gap-x-6 gap-y-3">
            <DetailMeta label={t("name")}>{registered.name}</DetailMeta>
            <DetailMeta label={tx("key")}>
              <span className="font-mono break-all">{registered.key}</span>
            </DetailMeta>
            <DetailMeta label={tx("version")}>{registered.version}</DetailMeta>
            <DetailMeta label={tx("status")}>
              <Badge tone={registered.status === "pending_review" ? "warning" : "neutral"}>
                {t(`statuses.${registered.status}`)}
              </Badge>
            </DetailMeta>
          </dl>
          <DialogActions>
            <Button onClick={close}>{t("done")}</Button>
          </DialogActions>
        </>
      ) : (
        <form noValidate method="post" onSubmit={handleSubmit(onSubmit)} className="space-y-3">
          {formError && <Alert>{formError}</Alert>}
          <TextField
            label={t("manifestUrl")}
            hint={t("manifestUrlHint")}
            type="url"
            placeholder="https://example.com/manifest.yaml"
            autoFocus
            spellCheck={false}
            error={errors.manifestUrl?.message}
            {...register("manifestUrl")}
          />
          {/* PLATFORM_KEY disabled for now */}
          {/* Not an account password: keep browsers and password managers from saving it. */}
          {
            // <PasswordField
            //   label={t("platformKey")}
            //   hint={t("platformKeyHint")}
            //   autoComplete="one-time-code"
            //   autoCapitalize="none"
            //   spellCheck={false}
            //   data-1p-ignore
            //   data-lpignore="true"
            //   data-bwignore
            //   error={errors.platformKey?.message}
            //   {...register("platformKey")}
            // />
          }
          <DialogActions>
            <Button variant="secondary" onClick={close} disabled={isSubmitting}>
              {tc("cancel")}
            </Button>
            <Button type="submit" loading={isSubmitting}>
              {t("submit")}
            </Button>
          </DialogActions>
        </form>
      )}
    </Dialog>
  );
}
