use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;

/// `POST /v1/iam/extensions` (`ExtensionInput`).
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct RegisterExtensionRequest {
    #[validate(length(
        min = 1,
        max = 2048,
        code = "invalid_manifest_url",
        message = "`manifest_url` is required, at most 2048 characters."
    ))]
    pub manifest_url: String,
}

/// `POST /v1/iam/extensions/{id}/reject` (`ExtensionRejectInput`).
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct RejectExtensionRequest {
    #[validate(
        length(
            max = 2000,
            code = "invalid_reason",
            message = "`reason` can be at most 2000 characters."
        ),
        custom(function = "not_blank")
    )]
    pub reason: String,
}

fn not_blank(value: &str) -> Result<(), validator::ValidationError> {
    if value.trim().is_empty() {
        return Err(validator::ValidationError::new("invalid_reason")
            .with_message("`reason` is required.".into()));
    }
    Ok(())
}
