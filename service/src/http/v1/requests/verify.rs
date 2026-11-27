use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;

/// `POST /v1/auth/verify-email`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct VerifyEmailRequest {
    #[validate(length(min = 1))]
    pub token: String,
}

/// `POST /v1/auth/google`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct GoogleSignInRequest {
    #[validate(length(min = 1))]
    pub id_token: String,
}

/// `POST /v1/auth/phone/code`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct PhoneCodeRequest {
    #[validate(length(min = 1))]
    pub phone: String,
    #[validate(custom(function = "super::validation::rules::channel"))]
    pub channel: String,
}

/// `POST /v1/auth/phone/verify`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct PhoneVerifyRequest {
    #[validate(length(min = 1))]
    pub phone: String,
    #[validate(length(min = 1))]
    pub code: String,
}
