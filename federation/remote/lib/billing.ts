import type { SeatPrice, Subscription } from "@zerpto/sdk";
import Decimal from "decimal.js";

/** The free trial: one per person (across every workspace), with every feature. */
export const TRIAL_DAYS = 30;
/** Seats a trial allows, counting the owner and pending invites. */
export const TRIAL_SEATS = 3;
/** Days an unpaid renewal keeps full access past its due date before the workspace is locked. */
export const GRACE_DAYS = 3;
/** Days before the trial ends from which the app starts reminding. */
export const TRIAL_REMINDER_DAYS = 7;
/** Billing cut-offs are all in this timezone. */
export const BILLING_TIME_ZONE = "Asia/Jakarta";

export type SubscriptionStatus = Subscription["status"];

/** Paid features work during the trial and while paid; `free` and `overdue` lock them. */
export function paidFeaturesUnlocked(status: SubscriptionStatus) {
  return status === "trial" || status === "paid";
}

/** Seats left before an invite needs more seats. */
export function freeSeats(subscription: Pick<Subscription, "purchasedSeats" | "occupiedSeats">) {
  return Math.max(0, subscription.purchasedSeats - subscription.occupiedSeats);
}

/** The price list's entry for a currency, else the first (phase 1 has only IDR). */
export function pickSeatPrice(prices: SeatPrice[], currency = "IDR") {
  return prices.find((price) => price.currency === currency) ?? prices[0];
}

/** Seats × the seat price, PPN included. Round half up once, on the final amount. */
export function seatsTotal(price: SeatPrice, seats: number) {
  return roundMoney(new Decimal(price.seatPrice).times(seats), price.currency);
}

/** Days between two date-only values (local midnight `Date`s from the SDK), ignoring DST shifts. */
function daysBetween(from: Date, to: Date) {
  const utc = (d: Date) => Date.UTC(d.getFullYear(), d.getMonth(), d.getDate());
  return Math.round((utc(to) - utc(from)) / 86_400_000);
}

/** Today in Asia/Jakarta as a local-midnight `Date`, comparable with the SDK's date-only values. */
export function billingToday(now = new Date()) {
  const [y, m, d] = new Intl.DateTimeFormat("en-CA", { timeZone: BILLING_TIME_ZONE })
    .format(now)
    .split("-")
    .map(Number);
  return new Date(y, m - 1, d);
}

/**
 * Estimate of a mid-period seat addition: seats × price × days remaining ÷ days in the period.
 * Days remaining run from today up to, but not including, the next due date. The API's invoice
 * has the exact amount; this is for the dialog before it is created.
 */
export function prorateSeats(
  price: SeatPrice,
  seats: number,
  periodStart: Date,
  periodEnd: Date,
  now = new Date(),
) {
  const daysInCycle = Math.max(1, daysBetween(periodStart, periodEnd));
  const daysRemaining = Math.min(
    daysInCycle,
    Math.max(0, daysBetween(billingToday(now), periodEnd)),
  );
  const amount = roundMoney(
    new Decimal(price.seatPrice).times(seats).times(daysRemaining).div(daysInCycle),
    price.currency,
  );
  return { amount, daysRemaining, daysInCycle };
}

/**
 * Rounds half up to the currency's minor unit. IDR is billed in whole rupiah (ISO 4217 lists 2
 * decimals for it, so it is special-cased); other currencies follow Intl.
 */
function roundMoney(value: Decimal, currency: string) {
  const digits =
    currency === "IDR"
      ? 0
      : (new Intl.NumberFormat("en", { style: "currency", currency }).resolvedOptions()
          .maximumFractionDigits ?? 2);
  return value.toDecimalPlaces(digits, Decimal.ROUND_HALF_UP).toString();
}

/**
 * The due date of an unpaid renewal: `locksAt` is 00:00 Asia/Jakarta, `GRACE_DAYS` after it. The
 * "payment due" warning starts then; before that, the renewal invoice is simply waiting.
 */
export function renewalDueAt(locksAt: Date) {
  return new Date(locksAt.getTime() - GRACE_DAYS * 86_400_000);
}
