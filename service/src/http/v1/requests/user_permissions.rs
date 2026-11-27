use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;
use zerpto_core::http::{filters::QueryParam, problem::Problem};

use crate::models::user_permission_grants::NewGrant;

/// One entry of `PUT /v1/iam/users/{id}/permissions`.
#[derive(Debug, Deserialize, Serialize, TS)]
pub struct GrantRequest {
    pub permission: String,
    /// Left out or `null` for a grant that does not expire.
    #[serde(default)]
    pub expires_at: Option<String>,
}

/// `PUT /v1/iam/users/{id}/permissions` (`UserPermissionsInput`).
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct SetUserPermissionsRequest {
    pub grants: Vec<GrantRequest>,
}

impl SetUserPermissionsRequest {
    /// Parses each `expires_at` into a timestamp.
    ///
    /// # Errors
    ///
    /// `400` when an `expires_at` is not an RFC 3339 timestamp.
    pub fn into_domain(self) -> Result<Vec<NewGrant>, Problem> {
        self.grants
            .into_iter()
            .map(|grant| {
                let expires_at = QueryParam::new(
                    grant.expires_at.as_deref(),
                    "expires_at",
                    "invalid_expires_at",
                )
                .timestamp()?;
                Ok(NewGrant {
                    permission: grant.permission,
                    expires_at,
                })
            })
            .collect()
    }
}
