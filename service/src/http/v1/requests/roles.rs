use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;
use zerpto_core::{
    helpers::non_blank,
    http::{nullable::Nullable, problem::Problem},
};

use crate::models::roles::Changes;

const DESCRIPTION_MAX_LENGTH: usize = 255;

fn check_description(value: &str) -> Result<String, Problem> {
    let value = value.trim();
    if value.chars().count() > DESCRIPTION_MAX_LENGTH {
        return Err(Problem::unprocessable(
            "invalid_description",
            format!("`description` can be at most {DESCRIPTION_MAX_LENGTH} characters."),
        )
        .field("description"));
    }
    Ok(value.to_string())
}

/// `POST /v1/iam/roles` (`RoleInput` in the API contract).
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct CreateRoleRequest {
    #[validate(custom(function = "zerpto_core::validation::rules::name"))]
    pub name: String,
    #[validate(length(
        max = 255,
        code = "invalid_description",
        message = "`description` can be at most 255 characters."
    ))]
    #[serde(default)]
    pub description: Option<String>,
    /// Required unless `clone_from_role_id` is given.
    #[serde(default)]
    pub permissions: Option<Vec<String>>,
    #[serde(default)]
    pub clone_from_role_id: Option<String>,
}

/// A checked [`CreateRoleRequest`].
#[derive(Debug)]
pub struct NewRole {
    pub name: String,
    pub description: Option<String>,
    pub permissions: Option<Vec<String>>,
    pub clone_from: Option<String>,
}

impl CreateRoleRequest {
    /// Trims the checked body into what the model takes.
    #[must_use]
    pub fn into_domain(self) -> NewRole {
        NewRole {
            name: self.name.trim().to_string(),
            description: non_blank(self.description),
            permissions: self.permissions,
            clone_from: self.clone_from_role_id,
        }
    }
}

/// `PATCH /v1/iam/roles/{id}` (`RoleUpdate`): only the fields sent change.
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct UpdateRoleRequest {
    #[validate(custom(function = "zerpto_core::validation::rules::name"))]
    pub name: Option<String>,
    /// `null` clears it.
    #[serde(default)]
    #[ts(optional, type = "string | null")]
    pub description: Nullable<String>,
    pub permissions: Option<Vec<String>>,
}

impl UpdateRoleRequest {
    /// Trims the checked body into the changes the model applies.
    ///
    /// # Errors
    ///
    /// `422` for a too long description.
    pub fn into_changes(self) -> Result<Changes, Problem> {
        let description = match self.description.into_change() {
            None => None,
            Some(None) => Some(None),
            Some(Some(value)) => Some(
                non_blank(Some(value))
                    .map(|value| check_description(&value))
                    .transpose()?,
            ),
        };
        Ok(Changes {
            name: self.name.map(|name| name.trim().to_string()),
            description,
            permissions: self.permissions,
        })
    }
}
