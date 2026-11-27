use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::models::{members::Member, plan};

/// A person in the tenant (`User` in the API contract): the id is the
/// membership's (`mem-`).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct User {
    pub id: String,
    pub email: String,
    /// Null while the user is `invited`.
    pub name: Option<String>,
    pub role_ids: Vec<String>,
    /// `invited`, `active` or `deactivated`.
    pub status: String,
    /// `read_only` when the company is on the free plan or overdue and this
    /// person is not an owner; otherwise `full`.
    pub access: String,
    pub created_at: DateTime<Utc>,
}

impl User {
    /// Builds the user for a tenant whose paid features are `locked` (or not).
    /// `owner_role_id` is the API id of the built-in owner role.
    #[must_use]
    pub fn new(member: Member, locked: bool, owner_role_id: &str) -> Self {
        let owner = member.role_ids.iter().any(|id| id == owner_role_id);
        Self {
            id: member.api_id(),
            email: member.email,
            name: member.name,
            role_ids: member.role_ids,
            status: member.status.to_string(),
            access: plan::access(locked, owner).to_string(),
            created_at: member.created_at,
        }
    }
}
