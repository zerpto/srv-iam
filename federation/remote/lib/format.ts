import { fromDate, parseDateTime, toCalendarDateTime, toZoned } from "@internationalized/date";
import { parseDate } from "@zerpto/sdk";
import Decimal from "decimal.js";
import { useFormatter } from "next-intl";
import { useCallback } from "react";

/**
 * Formats API money (decimal strings) for display, e.g. "Rp 1.500.000" or "Rp 12.500,50".
 * Whole amounts drop the ",00". Display only: do arithmetic with `Decimal`, not on this output.
 * Negative amounts (credits, net loss) keep their sign.
 */
export function useFormatMoney() {
  const format = useFormatter();
  return useCallback(
    (amount: string | number | Decimal, currency: string) => {
      const value = new Decimal(amount);
      return format.number(value.toNumber(), {
        style: "currency",
        currency,
        minimumFractionDigits: value.isInteger() ? 0 : 2,
        maximumFractionDigits: 2,
      });
    },
    [format],
  );
}

/**
 * Formats a date-only value: the SDK's `Date` (local midnight of the stated day), or a
 * `YYYY-MM-DD` string from a date input (read the same way, so no day shift in any timezone).
 */
export function useFormatDay() {
  const format = useFormatter();
  return useCallback(
    (date: Date | string, style: "medium" | "short" | "long" = "medium") =>
      format.dateTime(typeof date === "string" ? parseDate(date) : date, { dateStyle: style }),
    [format],
  );
}

/** A `YYYY-MM-DD` string (date input, `today()`) as the `Date` the SDK expects for date fields. */
export const toDate = parseDate;

/** Like `toDate`, for an optional date input (empty string means "not set"). */
export function optDate(value: string | null | undefined) {
  return value ? parseDate(value) : undefined;
}

/** The last moment of a calendar day (the SDK's date-only `Date` is its local midnight). */
export function endOfDay(date: Date) {
  const end = new Date(date);
  end.setHours(23, 59, 59, 999);
  return end;
}

/** A date as a `YYYY-MM-DD` string in local time: for comparing, sorting and React keys. */
export function dayKey(date: Date | string) {
  return typeof date === "string" ? date : toDateInput(date);
}

/** Today as `YYYY-MM-DD` in the browser's timezone (default for date inputs and "as of"). */
export function today() {
  return toDateInput(new Date());
}

/** Now as `YYYY-MM-DDTHH:mm:ss` in the given (tenant) timezone: default for a date-time input. */
export function nowInput(timeZone: string) {
  return toDateTimeInput(new Date(), timeZone);
}

/** An instant as `YYYY-MM-DDTHH:mm:ss` on the wall clock of `timeZone` (the tenant's). */
export function toDateTimeInput(date: Date, timeZone: string) {
  return toCalendarDateTime(fromDate(date, timeZone)).toString().slice(0, 19);
}

/**
 * A date-time input value, read as wall-clock time in `timeZone` (the tenant's), as the instant
 * the SDK sends (serialized to UTC).
 */
export function toDateTime(value: string, timeZone: string) {
  return toZoned(parseDateTime(value), timeZone).toDate();
}

export function toDateInput(date: Date) {
  const offset = date.getTimezoneOffset() * 60_000;
  return new Date(date.getTime() - offset).toISOString().slice(0, 10);
}

export function addDays(date: string, days: number) {
  const next = new Date(`${date}T00:00:00Z`);
  next.setUTCDate(next.getUTCDate() + days);
  return next.toISOString().slice(0, 10);
}

export function startOfMonth(date = today()) {
  return `${date.slice(0, 7)}-01`;
}

/**
 * Tax rates come as a decimal string. The spec doesn't say whether "PPN 11%" is `11` or `0.11`,
 * so values below 1 are read as fractions. Revisit when the spec says.
 */
export function ratePercent(rate: string) {
  const value = new Decimal(rate);
  return value.gt(0) && value.lt(1) ? value.times(100) : value;
}

/** Date ranges for report presets, as `YYYY-MM-DD` strings. */
export function rangePreset(
  preset: "thisMonth" | "lastMonth" | "thisQuarter" | "thisYear" | "lastYear",
  now = today(),
) {
  const year = Number(now.slice(0, 4));
  const month = Number(now.slice(5, 7));
  const pad = (n: number) => String(n).padStart(2, "0");
  const lastDay = (y: number, m: number) => new Date(Date.UTC(y, m, 0)).getUTCDate();
  switch (preset) {
    case "thisMonth":
      return {
        start: `${year}-${pad(month)}-01`,
        end: `${year}-${pad(month)}-${lastDay(year, month)}`,
      };
    case "lastMonth": {
      const y = month === 1 ? year - 1 : year;
      const m = month === 1 ? 12 : month - 1;
      return { start: `${y}-${pad(m)}-01`, end: `${y}-${pad(m)}-${lastDay(y, m)}` };
    }
    case "thisQuarter": {
      const first = Math.floor((month - 1) / 3) * 3 + 1;
      return {
        start: `${year}-${pad(first)}-01`,
        end: `${year}-${pad(first + 2)}-${lastDay(year, first + 2)}`,
      };
    }
    case "thisYear":
      return { start: `${year}-01-01`, end: `${year}-12-31` };
    case "lastYear":
      return { start: `${year - 1}-01-01`, end: `${year - 1}-12-31` };
  }
}

export function asOfPreset(preset: "today" | "lastMonthEnd" | "lastYearEnd", now = today()) {
  if (preset === "today") return now;
  if (preset === "lastMonthEnd") return rangePreset("lastMonth", now).end;
  return rangePreset("lastYear", now).end;
}
