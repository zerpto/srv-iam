import { useFormatter, useLocale } from "next-intl";
import { useMemo } from "react";
import type { SelectOption } from "@/components/ui/select-field";

export const DEFAULT_CURRENCY = "IDR";
export const DEFAULT_TIMEZONE = "Asia/Jakarta";
export const DEFAULT_FISCAL_MONTH = 1;

/** Localized month names, 1 = January. */
export function useMonthOptions(): SelectOption<number>[] {
  const format = useFormatter();
  return useMemo(
    () =>
      Array.from({ length: 12 }, (_, i) => ({
        value: i + 1,
        label: format.dateTime(new Date(2024, i, 1), { month: "long" }),
      })),
    [format],
  );
}

/** Used if the browser can't list currencies (very old engines). */
const FALLBACK_CURRENCIES = ["AUD", "CNY", "EUR", "GBP", "IDR", "JPY", "MYR", "SGD", "USD"];

/**
 * Every ISO 4217 currency the browser knows, by code, with localized names:
 * "IDR · Indonesian Rupiah". The API accepts any three-letter code.
 */
export function useCurrencyOptions(): SelectOption<string>[] {
  const locale = useLocale();
  return useMemo(() => {
    const names = new Intl.DisplayNames([locale], { type: "currency" });
    const codes =
      typeof Intl.supportedValuesOf === "function"
        ? Intl.supportedValuesOf("currency")
        : FALLBACK_CURRENCIES;
    return codes.map((code) => ({ value: code, label: `${code} · ${names.of(code) ?? code}` }));
  }, [locale]);
}
