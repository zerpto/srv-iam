import { z } from "zod";
import { type AddressValue, isAddressStarted, type PickedRegion } from "@/lib/address";
import type { ValidationT } from "./auth";

/** NPWP: 15 digits (legacy) or 16 (NIK-based). Punctuation is stripped before sending. */
export function normalizeTaxId(value: string) {
  return value.replace(/\D/g, "");
}

const optionalText = z.string().trim();

/** NPWP is optional; when given it is 15 or 16 digits (punctuation ignored). */
function taxIdField(t: ValidationT) {
  return optionalText.refine((value) => {
    const digits = normalizeTaxId(value);
    return digits.length === 0 || digits.length === 15 || digits.length === 16;
  }, t("taxIdInvalid"));
}

/** The optional address section: once started it needs a country (and the depth that country asks for). */
export function createAddressSchema(t: ValidationT) {
  return z
    .object({
      regionId: z.string(),
      country: z.string(),
      province: z.string(),
      district: z.string(),
      subDistrict: z.string(),
      village: z.string(),
      postalCode: z.string(),
      line1: z.string(),
      line2: z.string(),
      picked: z.array(z.custom<PickedRegion>()),
      incompleteLevel: z.enum(["", "country", "province", "district", "subDistrict", "village"]),
    })
    .superRefine((address: AddressValue, ctx) => {
      if (!isAddressStarted(address)) return;
      if (!address.country.trim()) {
        ctx.addIssue({ code: "custom", path: ["country"], message: t("addressCountryRequired") });
      }
      // The country asks for more depth and there are regions left to choose from.
      if (address.incompleteLevel) {
        ctx.addIssue({
          code: "custom",
          path: [address.incompleteLevel],
          message: t("addressPickLevel"),
        });
      }
    });
}

export function createTenantSchema(t: ValidationT) {
  return z.object({
    name: z.string().trim().min(2, t("workspaceNameRequired")),
    baseCurrency: z.string().regex(/^[A-Z]{3}$/, t("currencyRequired")),
    timezone: z.string().min(1, t("timezoneRequired")),
    // 0 = not chosen yet.
    fiscalYearStartMonth: z.number().int().min(1, t("fiscalMonthRequired")).max(12),
  });
}

export function createWorkspaceSchema(t: ValidationT) {
  return z.object({
    name: z.string().trim().min(2, t("workspaceNameRequired")),
    legalName: optionalText,
    taxId: taxIdField(t),
    address: createAddressSchema(t),
    timezone: z.string().min(1),
    fiscalYearStartMonth: z.number().int().min(1).max(12),
  });
}

export function createInviteUserSchema(t: ValidationT) {
  return z.object({
    email: z
      .string()
      .trim()
      .min(1, t("emailRequired"))
      .pipe(z.email(t("emailInvalid"))),
    roleIds: z.array(z.string()).min(1, t("roleRequired")),
  });
}

export function createEditUserSchema(t: ValidationT) {
  return z.object({
    roleIds: z.array(z.string()).min(1, t("roleRequired")),
  });
}

/** Custom role: name, description and the permission keys picked on the page. */
export function createRoleSchema(t: ValidationT) {
  return z.object({
    name: z.string().trim().min(2, t("roleNameRequired")),
    description: z.string().trim(),
    cloneFrom: z.string(),
    permissions: z.array(z.string()),
  });
}

export function createApiKeySchema(t: ValidationT) {
  return z.object({
    name: z.string().trim().min(1, t("keyNameRequired")),
    mode: z.enum(["live", "test"]),
  });
}

export type TenantValues = z.infer<ReturnType<typeof createTenantSchema>>;
export type WorkspaceValues = z.infer<ReturnType<typeof createWorkspaceSchema>>;
export type RoleValues = z.infer<ReturnType<typeof createRoleSchema>>;
export type InviteUserValues = z.infer<ReturnType<typeof createInviteUserSchema>>;
export type EditUserValues = z.infer<ReturnType<typeof createEditUserSchema>>;
export type ApiKeyValues = z.infer<ReturnType<typeof createApiKeySchema>>;

/**
 * `ExtensionInput`.
 * PLATFORM_KEY disabled for now: also the operator's platform key (sent as a header, so visible ASCII only).
 */
export function createRegisterExtensionSchema(t: ValidationT) {
  return z.object({
    manifestUrl: z
      .string()
      .trim()
      .min(1, t("manifestUrlRequired"))
      .max(2048, t("manifestUrlInvalid"))
      .pipe(z.url({ protocol: /^https?$/, error: t("manifestUrlInvalid") })),
    // PLATFORM_KEY disabled for now
    // platformKey: z
    //   .string()
    //   .trim()
    //   .min(1, t("platformKeyRequired"))
    //   .regex(/^[\x20-\x7e]*$/, t("platformKeyInvalid")),
  });
}

export type RegisterExtensionValues = z.infer<ReturnType<typeof createRegisterExtensionSchema>>;
