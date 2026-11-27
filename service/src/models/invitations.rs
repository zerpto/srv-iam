pub use super::_entities::invitations::{ActiveModel, Column, Entity, Model};
use chrono::Utc;
use loco_rs::prelude::*;
use sea_orm::TransactionTrait;
use uuid::Uuid;

use super::{
    digest::sha256_hex,
    memberships,
    users::{self, NewUser},
};

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
            self.updated_at = ActiveValue::Set(chrono::Utc::now().into());
        }
        Ok(self)
    }
}

/// Why an invitation token cannot be used.
#[derive(Debug, PartialEq, Eq)]
pub enum InvitationError {
    /// No invitation matches the token.
    Invalid,
    /// The invitation has expired or was already accepted.
    Expired,
}

impl Model {
    /// The pending invitation for a token from the invitation email.
    ///
    /// # Errors
    ///
    /// `Ok(Err(..))` says why the token is unusable; the outer error is a
    /// failed query.
    pub async fn find_by_token<C: ConnectionTrait>(
        db: &C,
        token: &str,
    ) -> ModelResult<Result<Self, InvitationError>> {
        let Some(invitation) = Entity::find()
            .filter(Column::TokenHash.eq(sha256_hex(token)))
            .one(db)
            .await?
        else {
            return Ok(Err(InvitationError::Invalid));
        };
        if invitation.accepted_at.is_some() || invitation.expires_at < Utc::now() {
            return Ok(Err(InvitationError::Expired));
        }
        Ok(Ok(invitation))
    }

    /// The role ids chosen in the invitation.
    #[must_use]
    pub fn role_uuids(&self) -> Vec<Uuid> {
        serde_json::from_value(self.role_ids.clone()).unwrap_or_default()
    }

    /// Creates the invited user (email already confirmed), joins them to the
    /// inviting tenant with the invited roles and marks the invitation
    /// accepted, in one transaction. The membership takes the invitation's id.
    ///
    /// # Errors
    ///
    /// `ModelError::EntityAlreadyExists` when the email already has an
    /// account; other errors when a write fails.
    pub async fn accept(
        self,
        db: &DatabaseConnection,
        name: &str,
        password: &str,
    ) -> ModelResult<users::Model> {
        let txn = db.begin().await?;
        let user = users::Model::create_with_password(
            &txn,
            &NewUser {
                email: &self.email,
                name,
                password,
                email_verified: true,
            },
        )
        .await?;
        memberships::Model::create_with_roles(
            &txn,
            self.id,
            user.id,
            self.tenant_id,
            &self.role_uuids(),
        )
        .await?;
        self.mark_accepted(&txn).await?;
        txn.commit().await?;
        Ok(user)
    }

    /// Joins an existing user to the inviting tenant and marks the invitation
    /// accepted. A member who is already active keeps what they have; one who
    /// was deactivated is reactivated with the invited roles.
    ///
    /// # Errors
    ///
    /// When a write fails.
    pub async fn accept_existing(
        self,
        db: &DatabaseConnection,
        user: &users::Model,
    ) -> ModelResult<memberships::Model> {
        let txn = db.begin().await?;
        let roles = self.role_uuids();
        let membership =
            match memberships::Model::find_in_tenant(&txn, user.id, self.tenant_id).await? {
                Some(existing) if existing.is_active() => existing,
                Some(existing) => {
                    let mut reactivated = existing.clone().into_active_model();
                    reactivated.deactivated_at = ActiveValue::set(None);
                    let membership = reactivated.update(&txn).await?;
                    membership.replace_roles(&txn, &roles).await?;
                    membership
                }
                None => {
                    memberships::Model::create_with_roles(
                        &txn,
                        self.id,
                        user.id,
                        self.tenant_id,
                        &roles,
                    )
                    .await?
                }
            };
        self.mark_accepted(&txn).await?;
        txn.commit().await?;
        Ok(membership)
    }

    async fn mark_accepted<C: ConnectionTrait>(self, db: &C) -> ModelResult<()> {
        let mut accepted = self.into_active_model();
        accepted.accepted_at = ActiveValue::set(Some(Utc::now().into()));
        accepted.update(db).await?;
        Ok(())
    }
}
