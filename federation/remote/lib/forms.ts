import type { FieldValues, Path, UseFormSetError } from "react-hook-form";
import type { ActionError } from "@/lib/api/result";

/**
 * Puts an API error on the matching form fields (a 422 lists one `details` entry per field, with
 * its own message) when possible. Returns the message to show in a form-level alert when nothing
 * could be placed on a field.
 */
export function applyActionError<T extends FieldValues>(
  error: ActionError,
  message: string,
  setError: UseFormSetError<T>,
  fieldMap: Partial<Record<string, Path<T>>>,
): string | null {
  const placed = (error.details ?? []).flatMap((detail) => {
    const name = detail.field ? fieldMap[detail.field] : undefined;
    return name ? [{ name, message: detail.message }] : [];
  });
  if (placed.length > 0) {
    placed.forEach(({ name, message: text }, index) =>
      setError(name, { type: "server", message: text }, { shouldFocus: index === 0 }),
    );
    return null;
  }
  const field = error.field ? fieldMap[error.field] : undefined;
  if (field) {
    setError(field, { type: "server", message }, { shouldFocus: true });
    return null;
  }
  return message;
}

export function newIdempotencyKey() {
  return crypto.randomUUID();
}

/**
 * Idempotency keys may only be reused to retry the *same* request after an unknown outcome
 * (network error, 5xx). A 4xx is a final answer: the API would replay it for the same key, so
 * the user's corrected resubmission needs a fresh key.
 */
export function shouldRotateIdempotencyKey(error: ActionError) {
  return error.status !== undefined && error.status < 500;
}
