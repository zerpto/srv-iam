use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::tenants::TenantSummary;
use crate::models::{scope::Scope, sessions::IssuedTokens, users};

#[derive(Debug, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum AccountStatus {
    PendingVerification,
    Active,
}

/// The signed-in person (`AuthUser` in the API contract).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct AuthUser {
    pub id: String,
    pub email: String,
    pub name: String,
}

impl From<&users::Model> for AuthUser {
    fn from(user: &users::Model) -> Self {
        Self {
            id: user.api_id(),
            email: user.email.clone(),
            name: user.name.clone(),
        }
    }
}

/// The account as it moves through sign-up (`AuthAccount`).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct AuthAccount {
    #[serde(flatten)]
    pub user: AuthUser,
    pub status: AccountStatus,
}

impl From<&users::Model> for AuthAccount {
    fn from(user: &users::Model) -> Self {
        Self {
            user: user.into(),
            status: if user.is_verified() {
                AccountStatus::Active
            } else {
                AccountStatus::PendingVerification
            },
        }
    }
}

/// A follow-up call the client may make.
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Action {
    pub name: String,
    pub href: String,
    pub method: String,
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct RegistrationMeta {
    pub actions: Vec<Action>,
}

impl RegistrationMeta {
    /// The only action after sign-up: log in once the email is confirmed.
    #[must_use]
    pub fn login() -> Self {
        Self {
            actions: vec![Action {
                name: "login".to_string(),
                href: "/v1/auth/sign-in".to_string(),
                method: "POST".to_string(),
            }],
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct TokenPair {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    #[ts(type = "number")]
    pub expires_in: u64,
}

impl From<IssuedTokens> for TokenPair {
    fn from(tokens: IssuedTokens) -> Self {
        Self {
            access_token: tokens.access_token,
            refresh_token: tokens.refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: tokens.expires_in,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct LoginResult {
    #[serde(flatten)]
    pub tokens: TokenPair,
    pub user: AuthUser,
    /// `null` until the user creates or joins a tenant (onboarding).
    pub tenant: Option<TenantSummary>,
}

impl LoginResult {
    #[must_use]
    pub fn new(tokens: IssuedTokens, user: &users::Model, scope: Option<&Scope>) -> Self {
        Self {
            tokens: tokens.into(),
            user: user.into(),
            tenant: scope.map(TenantSummary::from),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Me {
    pub user: AuthUser,
    /// `null` until the user creates or joins a tenant (onboarding).
    pub tenant: Option<TenantSummary>,
    pub permissions: Vec<String>,
    /// `read_only` when the company is on the free plan or overdue and this
    /// person is not an owner. Always `full` in the community edition.
    pub access: String,
    /// True while this person has never started a free trial in any tenant.
    /// Always `false` in the community edition, which has no trial.
    pub trial_available: bool,
    /// True once this person verified a phone number, which a trial needs.
    pub phone_verified: bool,
}
