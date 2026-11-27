use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;

/// `POST /v1/auth/accept-invite`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct AcceptInviteRequest {
    #[validate(length(min = 1))]
    pub token: String,
    #[validate(custom(function = "zerpto_core::validation::rules::name"))]
    pub name: String,
    #[validate(custom(function = "super::validation::rules::password"))]
    pub password: String,
}

/// `POST /v1/auth/accept-invite/existing`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct AcceptInviteExistingRequest {
    #[validate(length(min = 1))]
    pub token: String,
}
