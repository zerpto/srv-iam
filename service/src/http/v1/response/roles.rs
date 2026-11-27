use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::{
    permissions,
    plan::{self, Feature},
    roles::Detail,
};

/// A role (`Role` in the API contract).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Role {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    #[ts(rename = "type")]
    pub role_type: String,
    pub description: Option<String>,
    pub permissions: Vec<String>,
    #[ts(type = "number")]
    pub user_count: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Detail> for Role {
    fn from(detail: Detail) -> Self {
        let role = detail.role;
        Self {
            id: role.api_id(),
            permissions: role.permission_keys(),
            name: role.name,
            role_type: role.role_type,
            description: role.description,
            user_count: detail.user_count,
            created_at: role.created_at.with_timezone(&Utc),
            updated_at: role.updated_at.with_timezone(&Utc),
        }
    }
}

/// An entry of the permission catalogue.
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Permission {
    pub key: String,
    pub module: String,
    /// `read` or `write`.
    pub kind: String,
    /// `free` or `paid`: the lowest plan that includes the permission.
    pub minimum_plan: String,
    /// The tenant's plan does not include the permission yet.
    pub locked: bool,
    pub description: String,
}

impl Permission {
    /// `features` is `Catalogue::features`; `locked` says the tenant is free or
    /// overdue (false when the caller has no tenant yet).
    #[must_use]
    pub fn new(permission: &permissions::Permission, features: &[Feature], locked: bool) -> Self {
        let paid = features
            .iter()
            .any(|feature| feature.key == permission.key && feature.paid);
        Self {
            key: permission.key.clone(),
            module: permission.module.clone(),
            kind: if plan::is_write(features, &permission.key) {
                "write"
            } else {
                "read"
            }
            .to_string(),
            minimum_plan: if paid { "paid" } else { "free" }.to_string(),
            locked: paid && locked,
            description: permission.description.clone(),
        }
    }
}
