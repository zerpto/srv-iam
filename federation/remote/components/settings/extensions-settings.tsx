"use client";

import { Tab, TabGroup, TabList, TabPanel, TabPanels } from "@headlessui/react";
import { PlusIcon, PuzzlePieceIcon } from "@heroicons/react/24/outline";
import { useTranslations } from "next-intl";
import { useState } from "react";
import { Button } from "@/components/ui/button";
import { Can } from "@/components/ui/can";
import { Section } from "@/components/ui/detail";
import { EmptyState, PageContainer, PageHeader, QueryError } from "@/components/ui/page";
import { type Extension, useExtensions } from "@/lib/extensions";
import { useUrlParam } from "@/lib/use-url-state";
import { ExtensionCard, ExtensionCardsSkeleton, extensionGrid } from "./extension-card";
import { RegisterExtensionDialog } from "./register-extension-dialog";
import { useExtensionActions } from "./use-extension-actions";

const TABS = ["official", "partner", "community"] as const;

/**
 * The `party` value listed under each tab. The contract's enum only has `1st`; give the other tabs
 * their value once it defines one (until then no extension can belong to them).
 */
const PARTY: Record<(typeof TABS)[number], string | null> = {
  official: "1st",
  partner: null,
  community: null,
};

const tabClass =
  "relative -mb-px inline-flex items-center border-b-2 border-transparent px-1 py-2.5 text-sm font-medium whitespace-nowrap text-zinc-500 transition-colors hover:text-zinc-900 focus:outline-none data-focus:outline-2 data-focus:outline-offset-2 data-focus:outline-brand-600 data-selected:border-brand-600 data-selected:text-zinc-900 dark:text-zinc-400 dark:hover:text-white dark:data-selected:border-brand-500 dark:data-selected:text-white";

/**
 * GET /v1/iam/extensions: the approved extensions as cards, one tab per kind of maker, and which of
 * them this workspace uses. The open tab is kept in `?tab=`.
 */
export function ExtensionsSettings() {
  const t = useTranslations("extensions");
  // The API can't filter by party, so every page is loaded before the tabs sort the extensions out.
  const list = useExtensions();
  const actions = useExtensionActions(list.extensions);
  const [registering, setRegistering] = useState(false);
  const [tab, setTab] = useUrlParam<(typeof TABS)[number]>("tab", "official", TABS);

  function panel(extensions: Extension[], kind: (typeof TABS)[number]) {
    if (list.isError) return <QueryError error={list.error} onRetry={() => list.refetch()} />;
    if (!list.isComplete) return <ExtensionCardsSkeleton />;
    if (extensions.length === 0) {
      return (
        <EmptyState
          icon={PuzzlePieceIcon}
          title={t(`empty.${kind}.title`)}
          description={t(`empty.${kind}.body`)}
        />
      );
    }
    return (
      <Section.Content>
        <ul className={extensionGrid}>
          {extensions.map((extension) => (
            <li key={extension.id}>
              <ExtensionCard extension={extension} actions={actions} />
            </li>
          ))}
        </ul>
      </Section.Content>
    );
  }

  return (
    <PageContainer>
      <PageHeader
        title={t("title")}
        description={t("pageDescription")}
        actions={
          // For the platform operator (no tenant role identifies them).
          // PLATFORM_KEY disabled for now: the key is what the API checks.
          // Shown beside the page's other changes so it stays out of ordinary members' way.
          <Can>
            <Button variant="secondary" onClick={() => setRegistering(true)}>
              <PlusIcon aria-hidden="true" className="size-5" />
              {t("register.open")}
            </Button>
          </Can>
        }
      />

      <Section.Wrapper>
        <Section>
          <TabGroup selectedIndex={TABS.indexOf(tab)} onChange={(index) => setTab(TABS[index])}>
            <TabList
              aria-label={t("tabsLabel")}
              className="flex gap-6 overflow-x-auto border-b border-zinc-200 pr-(--gutter) pl-(--row-start) dark:border-white/10"
            >
              {TABS.map((kind) => (
                <Tab key={kind} className={tabClass}>
                  {t(`tabs.${kind}`)}
                </Tab>
              ))}
            </TabList>
            <TabPanels>
              {TABS.map((kind) => (
                <TabPanel key={kind} className="focus:outline-none">
                  {panel(
                    list.extensions.filter((extension) => extension.party === PARTY[kind]),
                    kind,
                  )}
                </TabPanel>
              ))}
            </TabPanels>
          </TabGroup>
        </Section>
      </Section.Wrapper>

      {actions.dialog}
      <RegisterExtensionDialog open={registering} onClose={() => setRegistering(false)} />
    </PageContainer>
  );
}
