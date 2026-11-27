import { useTranslations } from "next-intl";
import { useCallback } from "react";
import type { ActionError } from "./result";

/** Translates an API failure for display: known codes, rate limits, network errors, fallback. */
export function useApiErrorMessage() {
  const t = useTranslations("errors");
  const codes = useTranslations("errors.codes");

  return useCallback(
    (error: ActionError) => {
      if (error.code === "network_error") return t("network");
      if (error.status === 429) {
        return error.retryAfter
          ? t("rateLimited", { seconds: error.retryAfter })
          : t("rateLimitedGeneric");
      }
      if (error.status === 403 && !codes.has(error.code as Parameters<typeof codes>[0])) {
        return t("forbidden");
      }
      // Codes come from the API at runtime; only those listed in `errors.codes` are translated.
      const code = error.code as Parameters<typeof codes>[0];
      if (codes.has(code)) return codes(code);
      // A failed validation without a field of its own: say what was wrong.
      const detail = error.details?.length === 1 ? error.details[0].message : undefined;
      return detail || error.message || t("unexpected");
    },
    [t, codes],
  );
}
