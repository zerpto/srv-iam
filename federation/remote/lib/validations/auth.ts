import type { useTranslations } from "next-intl";
import { z } from "zod";

/** Mirrors `minLength: 12` on new passwords in the auth spec. */
export const MIN_PASSWORD_LENGTH = 8;

/** Validation messages are translated, so schemas are built per render with `t`. */
export type ValidationT = ReturnType<typeof useTranslations<"validation">>;

function fields(t: ValidationT) {
  return {
    email: z
      .string()
      .trim()
      .min(1, t("emailRequired"))
      .pipe(z.email(t("emailInvalid"))),
    name: z.string().trim().min(2, t("nameRequired")),
    newPassword: z
      .string()
      .min(MIN_PASSWORD_LENGTH, t("passwordMin", { min: MIN_PASSWORD_LENGTH })),
    confirmPassword: z.string().min(1, t("confirmRequired")),
  };
}

function passwordsMatch(
  t: ValidationT,
): [
  (data: { password: string; confirmPassword: string }) => boolean,
  { path: string[]; error: string },
] {
  return [
    (data) => data.password === data.confirmPassword,
    { path: ["confirmPassword"], error: t("passwordsMismatch") },
  ];
}

export function createLoginSchema(t: ValidationT) {
  return z.object({
    email: fields(t).email,
    password: z
      .string()
      .min(1, t("passwordRequired"))
      .min(MIN_PASSWORD_LENGTH, t("passwordMin", { min: MIN_PASSWORD_LENGTH })),
    remember: z.boolean(),
  });
}

export function createRegisterSchema(t: ValidationT) {
  const f = fields(t);
  return z.object({
    name: f.name,
    email: f.email,
    password: f.newPassword,
    /** UI-only consent; not sent to the API. */
    terms: z.boolean().refine((value) => value, t("termsRequired")),
  });
}

export function createForgotPasswordSchema(t: ValidationT) {
  return z.object({ email: fields(t).email });
}

export function createResetPasswordSchema(t: ValidationT) {
  const f = fields(t);
  return z
    .object({ password: f.newPassword, confirmPassword: f.confirmPassword })
    .refine(...passwordsMatch(t));
}

export function createAcceptInviteSchema(t: ValidationT) {
  const f = fields(t);
  return z
    .object({ name: f.name, password: f.newPassword, confirmPassword: f.confirmPassword })
    .refine(...passwordsMatch(t));
}

export function createChangePasswordSchema(t: ValidationT) {
  const f = fields(t);
  return z
    .object({
      currentPassword: z.string().min(1, t("passwordRequired")),
      password: f.newPassword,
      confirmPassword: f.confirmPassword,
    })
    .refine(...passwordsMatch(t));
}

export type LoginValues = z.infer<ReturnType<typeof createLoginSchema>>;
export type RegisterValues = z.infer<ReturnType<typeof createRegisterSchema>>;
export type ForgotPasswordValues = z.infer<ReturnType<typeof createForgotPasswordSchema>>;
export type ResetPasswordValues = z.infer<ReturnType<typeof createResetPasswordSchema>>;
export type AcceptInviteValues = z.infer<ReturnType<typeof createAcceptInviteSchema>>;
export type ChangePasswordValues = z.infer<ReturnType<typeof createChangePasswordSchema>>;
