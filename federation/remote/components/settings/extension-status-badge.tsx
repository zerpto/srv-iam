"use client";

import { useTranslations } from "next-intl";
import { Badge } from "@/components/ui/badge";
import type { Extension } from "@/lib/extensions";

/** Always on (core), enabled or disabled for this workspace. */
export function ExtensionStatusBadge({ extension }: { extension: Extension }) {
  const t = useTranslations("extensions");
  if (extension.core) return <Badge tone="brand">{t("alwaysOn")}</Badge>;
  return (
    <Badge tone={extension.enabled ? "success" : "neutral"}>
      {extension.enabled ? t("enabled") : t("disabled")}
    </Badge>
  );
}
