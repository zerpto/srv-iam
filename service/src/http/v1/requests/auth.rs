use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;

/// `POST /v1/auth/sign-up`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct SignUpRequest {
    #[validate(custom(function = "zerpto_core::validation::rules::name"))]
    pub name: String,
    #[validate(email(code = "invalid_email", message = "Email address is not valid."))]
    pub email: String,
    #[validate(custom(function = "super::validation::rules::password"))]
    pub password: String,
}

/// `POST /v1/auth/sign-in`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct SignInRequest {
    #[validate(length(min = 1))]
    pub email: String,
    #[validate(length(min = 1))]
    pub password: String,
}

/// `POST /v1/auth/refresh`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct RefreshRequest {
    #[validate(length(min = 1))]
    pub refresh_token: String,
}
