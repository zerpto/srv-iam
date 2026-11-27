/** The little the team screen needs from web.app's billing screens (the seat limit). */
import { useApiQuery } from "@/lib/api/query";
import { sdk } from "@/lib/api/sdk";

export const BILLING_PATH = "/settings/billing";
export const SUBSCRIPTION_KEY = ["get", "/v1/billing/subscription"];

/** GET /v1/billing/subscription: status (`free`, `trial`, `paid`, `overdue`), seats and dates. */
export function useSubscription({ enabled = true }: { enabled?: boolean } = {}) {
  const query = useApiQuery("/v1/billing/subscription", {}, () => sdk.billing.getSubscription(), {
    enabled,
  });
  return { ...query, subscription: query.data };
}
