pub use super::_entities::user_permission_grants::{ActiveModel, Column, Entity, Model};
use std::collections::HashMap;

use chrono::{DateTime, Utc};
use loco_rs::prelude::*;
use sea_orm::TransactionTrait;
use uuid::Uuid;
use zerpto_core::http::ids::ApiIds;

use super::{_entities::users, memberships, permissions::Catalogue, roles::grants};
use crate::http::ids::USER;

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert && self.id.is_not_set() {
            self.id = ActiveValue::Set(Uuid::now_v7());
        }
        if !insert && self.updated_at.is_unchanged() {
            self.updated_at = ActiveValue::Set(Utc::now().into());
        }
        Ok(self)
    }
}

/// A permission the request asks to give.
#[derive(Debug, Clone)]
pub struct NewGrant {
    pub permission: String,
    pub expires_at: Option<DateTime<Utc>>,
}

/// A grant as the API shows it.
#[derive(Debug, Clone)]
pub struct Grant {
    pub permission: String,
    pub expires_at: Option<DateTime<Utc>>,
    /// API id of the user who gave it, when they still exist.
    pub granted_by: Option<String>,
    pub granted_at: DateTime<Utc>,
}

/// Why direct grants could not be set.
#[derive(Debug, PartialEq, Eq)]
pub enum GrantError {
    UnknownPermission(String),
    CannotGrant(String),
    /// A grant that expires must expire in the future.
    ExpiresInThePast(String),
}

impl Model {
    /// Whether the grant has not expired.
    #[must_use]
    pub fn is_current(&self) -> bool {
        self.expires_at.is_none_or(|at| at > Utc::now())
    }

    /// The permission keys a member holds directly right now.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn current_keys<C: ConnectionTrait>(
        db: &C,
        membership_id: Uuid,
    ) -> ModelResult<Vec<String>> {
        Ok(Entity::find()
            .filter(Column::MembershipId.eq(membership_id))
            .all(db)
            .await?
            .into_iter()
            .filter(Self::is_current)
            .map(|grant| grant.permission)
            .collect())
    }

    /// The grants a member holds right now, oldest first.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn current_for<C: ConnectionTrait>(
        db: &C,
        membership_id: Uuid,
    ) -> ModelResult<Vec<Grant>> {
        let mut rows: Vec<Self> = Entity::find()
            .filter(Column::MembershipId.eq(membership_id))
            .all(db)
            .await?
            .into_iter()
            .filter(Self::is_current)
            .collect();
        rows.sort_by(|a, b| (a.created_at, &a.permission).cmp(&(b.created_at, &b.permission)));
        let granters: HashMap<Uuid, String> = users::Entity::find()
            .filter(users::Column::Id.is_in(rows.iter().filter_map(|row| row.granted_by_user_id)))
            .all(db)
            .await?
            .into_iter()
            .map(|user| (user.id, ApiIds::new(USER).encode(&user.id)))
            .collect();
        Ok(rows
            .into_iter()
            .map(|row| Grant {
                granted_by: row
                    .granted_by_user_id
                    .and_then(|id| granters.get(&id).cloned()),
                expires_at: row.expires_at.map(|at| at.with_timezone(&Utc)),
                granted_at: row.created_at.with_timezone(&Utc),
                permission: row.permission,
            })
            .collect())
    }

    /// Replaces a member's direct grants with `grants_wanted`. A permission
    /// that was already granted keeps who gave it and when; only its expiry
    /// changes.
    ///
    /// # Errors
    ///
    /// `UnknownPermission`, `CannotGrant` (the caller must hold what they
    /// give) or `ExpiresInThePast`.
    pub async fn replace_for(
        db: &DatabaseConnection,
        membership: &memberships::Model,
        grants_wanted: &[NewGrant],
        caller: &users::Model,
        caller_permissions: &[String],
    ) -> ModelResult<Result<(), GrantError>> {
        let declared = Catalogue::declared(db).await?;
        for grant in grants_wanted {
            if !declared.is_known(&grant.permission) {
                return Ok(Err(GrantError::UnknownPermission(grant.permission.clone())));
            }
            if grant.expires_at.is_some_and(|at| at <= Utc::now()) {
                return Ok(Err(GrantError::ExpiresInThePast(grant.permission.clone())));
            }
            if !grants(caller_permissions, &grant.permission) {
                return Ok(Err(GrantError::CannotGrant(grant.permission.clone())));
            }
        }
        // The last entry for a permission wins.
        let mut wanted: Vec<&NewGrant> = Vec::new();
        for grant in grants_wanted.iter().rev() {
            if !wanted
                .iter()
                .any(|kept| kept.permission == grant.permission)
            {
                wanted.push(grant);
            }
        }

        let txn = db.begin().await?;
        let existing = Entity::find()
            .filter(Column::MembershipId.eq(membership.id))
            .all(&txn)
            .await?;
        for row in &existing {
            if !wanted
                .iter()
                .any(|grant| grant.permission == row.permission)
            {
                Entity::delete_by_id(row.id).exec(&txn).await?;
            }
        }
        for grant in wanted {
            let expires_at = grant.expires_at.map(Into::into);
            if let Some(row) = existing
                .iter()
                .find(|row| row.permission == grant.permission)
            {
                let mut changed = row.clone().into_active_model();
                changed.expires_at = ActiveValue::set(expires_at);
                changed.update(&txn).await?;
            } else {
                ActiveModel {
                    membership_id: ActiveValue::set(membership.id),
                    permission: ActiveValue::set(grant.permission.clone()),
                    expires_at: ActiveValue::set(expires_at),
                    granted_by_user_id: ActiveValue::set(Some(caller.id)),
                    ..Default::default()
                }
                .insert(&txn)
                .await?;
            }
        }
        txn.commit().await?;
        Ok(Ok(()))
    }
}
