use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;

/// `PUT /v1/auth/password`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct ChangePasswordRequest {
    #[validate(length(min = 1))]
    pub current_password: String,
    #[validate(custom(function = "super::validation::rules::password"))]
    pub new_password: String,
}

/// `POST /v1/auth/reset-password`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct ForgotPasswordRequest {
    #[validate(email(code = "invalid_email", message = "Email address is not valid."))]
    pub email: String,
}

/// `PUT /v1/auth/reset-password`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct ResetPasswordRequest {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(custom(function = "super::validation::rules::password"))]
    pub new_password: String,
}
