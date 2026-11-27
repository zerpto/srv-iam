use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;
use zerpto_core::validation::failure;

fn role_ids(value: &[String]) -> Result<(), validator::ValidationError> {
    if value.is_empty() {
        return Err(failure("unknown_role", "Give the user at least one role."));
    }
    Ok(())
}

/// `POST /v1/iam/users`: invite a person (`UserInput` in the API contract).
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct InviteUserRequest {
    #[validate(email(code = "invalid_email", message = "Email address is not valid."))]
    pub email: String,
    #[validate(custom(function = "role_ids"))]
    pub role_ids: Vec<String>,
}

/// `PATCH /v1/iam/users/{id}`: replace a user's roles (`UserUpdate`).
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct UpdateUserRequest {
    #[validate(custom(function = "role_ids"))]
    pub role_ids: Vec<String>,
}
