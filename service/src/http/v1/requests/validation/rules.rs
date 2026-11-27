use std::borrow::Cow;

use validator::ValidationError;
use zerpto_core::validation::failure;

/// A new password has at least this many characters (the API contract).
pub const PASSWORD_MIN_LENGTH: usize = 7;

/// A new password's length. Code `weak_password`.
///
/// # Errors
///
/// `weak_password` when shorter than [`PASSWORD_MIN_LENGTH`].
pub fn password(value: &str) -> Result<(), ValidationError> {
    if value.chars().count() < PASSWORD_MIN_LENGTH {
        return Err(
            ValidationError::new("weak_password").with_message(Cow::Owned(format!(
                "Password must be at least {PASSWORD_MIN_LENGTH} characters long."
            ))),
        );
    }
    Ok(())
}

/// A phone verification channel: `whatsapp` or `sms`. Code `invalid_channel`.
///
/// # Errors
///
/// `invalid_channel` for anything else.
pub fn channel(value: &str) -> Result<(), ValidationError> {
    if matches!(value, "whatsapp" | "sms") {
        Ok(())
    } else {
        Err(failure(
            "invalid_channel",
            "`channel` must be `whatsapp` or `sms`.",
        ))
    }
}

/// Trims and lower-cases an email address, the form it is stored and looked up in.
#[must_use]
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}
