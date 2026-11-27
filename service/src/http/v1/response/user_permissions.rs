use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::user_permission_grants::Grant;

/// A permission given to one user directly (`UserPermissionGrant`).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct UserPermissionGrant {
    pub permission: String,
    pub expires_at: Option<DateTime<Utc>>,
    pub granted_by: Option<String>,
    pub granted_at: DateTime<Utc>,
}

impl From<Grant> for UserPermissionGrant {
    fn from(grant: Grant) -> Self {
        Self {
            permission: grant.permission,
            expires_at: grant.expires_at,
            granted_by: grant.granted_by,
            granted_at: grant.granted_at,
        }
    }
}

/// What one user may do (`UserPermissions`).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct UserPermissions {
    pub direct: Vec<UserPermissionGrant>,
    pub effective: Vec<String>,
}
