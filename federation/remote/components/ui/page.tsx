"use client";

import { PageHeader as BasePageHeader, type PageHeaderProps } from "@zerpto/ui";

export {
  Card,
  EmptyState,
  PageContainer,
  PageFooter,
  QueryError,
  SettingsSection,
  TableSkeleton,
} from "@zerpto/ui";

/**
 * The shared title row. The host gives it the menu icon and draws the breadcrumbs from the URL, so
 * web.app's `crumbs` are accepted and ignored.
 */
export function PageHeader({
  crumbs,
  ...props
}: PageHeaderProps & { crumbs?: { label: string; href?: string }[] }) {
  void crumbs;
  return <BasePageHeader {...props} />;
}
