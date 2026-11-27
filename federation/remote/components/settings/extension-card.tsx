"use client";

import { useTranslations } from "next-intl";
import { useId } from "react";
import { Button } from "@/components/ui/button";
import { Can } from "@/components/ui/can";
import { Card } from "@/components/ui/page";
import { Bone } from "@/components/ui/skeletons";
import { TextLink } from "@/components/ui/text-link";
import type { Extension } from "@/lib/extensions";
import { ExtensionStatusBadge } from "./extension-status-badge";
import type { useExtensionActions } from "./use-extension-actions";

/** The responsive grid the cards sit in. */
export const extensionGrid = "grid gap-3 sm:grid-cols-2 xl:grid-cols-3 2xl:grid-cols-4";

/** One extension: name (linking to its detail page), state, and the enable / disable button. */
export function ExtensionCard({
  extension,
  actions,
}: {
  extension: Extension;
  actions: ReturnType<typeof useExtensionActions>;
}) {
  const t = useTranslations("extensions");
  const titleId = useId();

  return (
    <Card className="h-full">
      <article aria-labelledby={titleId} className="flex h-full flex-col gap-2 p-4">
        <div className="flex items-start justify-between gap-3">
          <h2
            id={titleId}
            className="min-w-0 text-sm font-semibold break-words text-zinc-900 dark:text-white"
          >
            <TextLink href={`/settings/extensions/${encodeURIComponent(extension.id)}`}>
              {extension.name}
            </TextLink>
          </h2>
          <ExtensionStatusBadge extension={extension} />
        </div>
        <p className="line-clamp-3 text-sm text-zinc-600 dark:text-zinc-400">
          {extension.description}
        </p>
        <div className="mt-auto flex min-h-9 items-end justify-between gap-3 pt-2">
          <p className="text-xs text-zinc-500 tabular-nums dark:text-zinc-400">
            {extension.core ? t("core") : t("optional")} ·{" "}
            {t("versionValue", { version: extension.version })}
          </p>
          {!extension.core && (
            <Can>
              {extension.enabled ? (
                <Button
                  variant="secondary"
                  size="sm"
                  aria-label={t("disableName", { name: extension.name })}
                  onClick={() => actions.askDisable(extension)}
                >
                  {t("disable")}
                </Button>
              ) : (
                <Button
                  size="sm"
                  aria-label={t("enableName", { name: extension.name })}
                  loading={actions.pendingId === extension.id}
                  disabled={actions.pendingId !== null}
                  onClick={() => void actions.enable(extension)}
                >
                  {t("enable")}
                </Button>
              )}
            </Can>
          )}
        </div>
      </article>
    </Card>
  );
}

/** Placeholder cards while the list loads. */
export function ExtensionCardsSkeleton() {
  return (
    <div aria-busy="true" className="animate-pulse">
      <div className="py-3 pr-(--gutter) pl-(--row-start)">
        <div className={extensionGrid}>
          {[0, 1, 2, 3].map((i) => (
            <Card key={i} className="space-y-3 p-4">
              <Bone className="h-4 w-32" />
              <Bone className="h-3 w-full" />
              <Bone className="h-3 w-2/3" />
              <Bone className="h-7 w-20" />
            </Card>
          ))}
        </div>
      </div>
    </div>
  );
}
