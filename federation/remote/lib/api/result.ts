import { FetchError, ResponseError, type ErrorErrorDetailsInner } from "@zerpto/sdk";

/** One problem of a failed validation (422): the request field at fault (null for the whole body). */
export type ErrorDetail = ErrorErrorDetailsInner;

/**
 * The raw error body, not parsed by the SDK, so snake_case. The spec is `{ error: { message,
 * request_id, details? } }` with no `code`: clients go by the HTTP status and `details`. `code` and
 * `field` are still read when an older API sends them.
 */
type ErrorEnvelope = {
  error: {
    message: string;
    request_id?: string;
    details?: ErrorDetail[];
    code?: string;
    field?: string | null;
  };
};

/** What a status means when the API sends no `code`. */
const STATUS_CODES: Record<number, string> = {
  400: "bad_request",
  401: "unauthorized",
  403: "forbidden",
  404: "not_found",
  409: "conflict",
  422: "unprocessable",
  429: "rate_limited",
};

/** Normalized failure. Branch on `status` (and `details`); show `useApiErrorMessage()` to users. */
export type ActionError = {
  ok: false;
  /** The API's `code` when it sends one, otherwise a name for the HTTP status (`conflict`). */
  code: string;
  /** The API's own message (English, may change). Fallback when `code` has no translation. */
  message: string;
  status?: number;
  /** What was wrong, per request field (a 422). */
  details?: ErrorDetail[];
  /** First request field at fault, e.g. `new_password`. */
  field?: string;
  retryAfter?: number;
  requestId?: string;
};

export type ActionResult<T = undefined> = { ok: true; data: T } | ActionError;

function isErrorEnvelope(value: unknown): value is ErrorEnvelope {
  return (
    typeof value === "object" &&
    value !== null &&
    "error" in value &&
    typeof (value as ErrorEnvelope).error?.message === "string"
  );
}

export function toActionError(error: unknown, response?: Response): ActionError {
  const envelope = isErrorEnvelope(error) ? error.error : undefined;
  const status = response?.status;

  if (status === 429) {
    return {
      ok: false,
      status,
      code: envelope?.code ?? STATUS_CODES[429],
      message: envelope?.message ?? "",
      retryAfter: Number(response?.headers.get("Retry-After")) || undefined,
      requestId: envelope?.request_id,
    };
  }

  if (envelope) {
    const details = envelope.details?.length ? envelope.details : undefined;
    return {
      ok: false,
      status,
      code: envelope.code ?? (status ? STATUS_CODES[status] : undefined) ?? "unexpected_error",
      message: envelope.message,
      details,
      field: envelope.field ?? details?.find((detail) => detail.field)?.field ?? undefined,
      requestId: envelope.request_id,
    };
  }

  return { ok: false, status, code: "unexpected_error", message: "" };
}

/**
 * The spec wraps every success response as `{ data, meta }`. The app sees the record itself, or
 * `{ data, pagination }` for a list; `meta` is dropped (read the SDK's raw response if an
 * endpoint needs its `links`, `web_urls` or `actions`).
 */
export type Unwrapped<T> = T extends { data: infer D; meta: infer M }
  ? D extends readonly unknown[]
    ? { data: D; pagination: M extends { pagination: infer P } ? P : undefined }
    : D
  : T;

export function unwrap<T>(response: T): Unwrapped<T> {
  const body = response as { data?: unknown; meta?: { pagination?: unknown } } | null | undefined;
  if (body === null || typeof body !== "object" || !("data" in body) || !("meta" in body)) {
    return response as Unwrapped<T>;
  }
  return (
    Array.isArray(body.data) ? { data: body.data, pagination: body.meta?.pagination } : body.data
  ) as Unwrapped<T>;
}

/** Turns anything the SDK can throw into an `ActionError`. */
export async function toFailure(cause: unknown): Promise<ActionError> {
  if (cause instanceof ResponseError) {
    const body: unknown = await cause.response
      .clone()
      .json()
      .catch(() => undefined);
    return toActionError(body, cause.response);
  }
  if (!(cause instanceof FetchError)) console.error("[api] request failed", cause);
  return { ok: false, code: "network_error", message: "" };
}

/** Runs an SDK call and normalizes success (unwrapped), API errors and network failures. */
export async function callApi<T>(send: () => Promise<T>): Promise<ActionResult<Unwrapped<T>>> {
  try {
    return { ok: true, data: unwrap(await send()) };
  } catch (cause) {
    return toFailure(cause);
  }
}

/** Like `callApi`, for TanStack Query: returns the unwrapped data and throws an `ActionError`. */
export async function queryApi<T>(send: () => Promise<T>): Promise<Unwrapped<T>> {
  const result = await callApi(send);
  if (!result.ok) throw result;
  return result.data;
}
