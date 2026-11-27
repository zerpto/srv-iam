pub use super::_entities::users::{ActiveModel, Column, Entity, Model};
use chrono::{Duration, Utc};
use loco_rs::{hash, prelude::*};
use sea_orm::SqlErr;
use uuid::Uuid;
use zerpto_core::http::ids::ApiIds;

use super::digest::sha256_hex;
use crate::http::ids::USER;

/// Hours an emailed verification link works.
pub const EMAIL_VERIFICATION_HOURS: i64 = 24;
/// Minutes an emailed password reset link works.
pub const RESET_TOKEN_MINUTES: i64 = 60;

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

/// What a new account is made from.
#[derive(Debug)]
pub struct NewUser<'a> {
    /// Trimmed and lower-cased already.
    pub email: &'a str,
    pub name: &'a str,
    pub password: &'a str,
    /// An invited or Google user has proven the address already.
    pub email_verified: bool,
}

fn hash_password(password: &str) -> ModelResult<String> {
    hash::hash_password(password).map_err(|e| ModelError::Any(e.into()))
}

impl Model {
    /// The API id: `usr-<uuid>`.
    #[must_use]
    pub fn api_id(&self) -> String {
        ApiIds::new(USER).encode(&self.id)
    }

    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: Uuid) -> ModelResult<Option<Self>> {
        Ok(Entity::find_by_id(id).one(db).await?)
    }

    /// `email` must be trimmed and lower-cased.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_email<C: ConnectionTrait>(
        db: &C,
        email: &str,
    ) -> ModelResult<Option<Self>> {
        Ok(Entity::find()
            .filter(Column::Email.eq(email))
            .one(db)
            .await?)
    }

    /// The user an emailed verification token was sent to.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_verification_token<C: ConnectionTrait>(
        db: &C,
        token: &str,
    ) -> ModelResult<Option<Self>> {
        Ok(Entity::find()
            .filter(Column::EmailVerificationToken.eq(sha256_hex(token)))
            .one(db)
            .await?)
    }

    /// The user a password reset token was sent to; check
    /// [`reset_expired`](Self::reset_expired) after.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_reset_token<C: ConnectionTrait>(
        db: &C,
        token: &str,
    ) -> ModelResult<Option<Self>> {
        Ok(Entity::find()
            .filter(Column::ResetToken.eq(sha256_hex(token)))
            .one(db)
            .await?)
    }

    /// Creates an account with a hashed password.
    ///
    /// # Errors
    ///
    /// `ModelError::EntityAlreadyExists` when the email is taken (also when
    /// two requests race for it); other errors when the write fails.
    pub async fn create_with_password<C: ConnectionTrait>(
        db: &C,
        new: &NewUser<'_>,
    ) -> ModelResult<Self> {
        let result = ActiveModel {
            email: ActiveValue::set(new.email.to_string()),
            name: ActiveValue::set(new.name.to_string()),
            password: ActiveValue::set(hash_password(new.password)?),
            email_verified_at: ActiveValue::set(new.email_verified.then(|| Utc::now().into())),
            ..Default::default()
        }
        .insert(db)
        .await;
        match result {
            Ok(user) => Ok(user),
            Err(err) if matches!(err.sql_err(), Some(SqlErr::UniqueConstraintViolation(_))) => {
                Err(ModelError::EntityAlreadyExists)
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Whether `password` is this user's password.
    #[must_use]
    pub fn verify_password(&self, password: &str) -> bool {
        hash::verify_password(password, &self.password)
    }

    #[must_use]
    pub const fn is_verified(&self) -> bool {
        self.email_verified_at.is_some()
    }

    #[must_use]
    pub const fn is_phone_verified(&self) -> bool {
        self.phone_verified_at.is_some()
    }

    /// Whether the password reset link has expired (or none was ever sent).
    #[must_use]
    pub fn reset_expired(&self) -> bool {
        self.reset_sent_at
            .is_none_or(|sent_at| sent_at + Duration::minutes(RESET_TOKEN_MINUTES) < Utc::now())
    }

    /// Whether the verification link has expired (or none was ever sent).
    #[must_use]
    pub fn verification_expired(&self) -> bool {
        self.email_verification_sent_at
            .is_none_or(|sent_at| sent_at + Duration::hours(EMAIL_VERIFICATION_HOURS) < Utc::now())
    }
}

impl ActiveModel {
    /// Makes a verification token and stores its hash. Returns the user and
    /// the token to put in the email; the token is not recoverable later.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn set_email_verification_sent<C: ConnectionTrait>(
        mut self,
        db: &C,
    ) -> ModelResult<(Model, String)> {
        let token = hash::random_string(48);
        self.email_verification_sent_at = ActiveValue::set(Some(Utc::now().into()));
        self.email_verification_token = ActiveValue::set(Some(sha256_hex(&token)));
        Ok((self.update(db).await?, token))
    }

    /// Records that the address is confirmed. The token is kept, so confirming
    /// twice is safe (the endpoint is idempotent by contract).
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn verified<C: ConnectionTrait>(mut self, db: &C) -> ModelResult<Model> {
        self.email_verified_at = ActiveValue::set(Some(Utc::now().into()));
        Ok(self.update(db).await?)
    }

    /// Sets a new password and refuses every session created before now
    /// (see `sessions_valid_after`).
    ///
    /// # Errors
    ///
    /// When hashing or the update fails.
    pub async fn change_password<C: ConnectionTrait>(
        mut self,
        db: &C,
        new_password: &str,
    ) -> ModelResult<Model> {
        self.password = ActiveValue::set(hash_password(new_password)?);
        self.sessions_valid_after = ActiveValue::set(Some(Utc::now().into()));
        Ok(self.update(db).await?)
    }

    /// Makes a password reset token and stores its hash. Returns the user and
    /// the token to put in the email; the token is not recoverable later.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn set_forgot_password_sent<C: ConnectionTrait>(
        mut self,
        db: &C,
    ) -> ModelResult<(Model, String)> {
        let token = hash::random_string(48);
        self.reset_sent_at = ActiveValue::set(Some(Utc::now().into()));
        self.reset_token = ActiveValue::set(Some(sha256_hex(&token)));
        Ok((self.update(db).await?, token))
    }

    /// Sets a new password with a reset token: the token stops working and
    /// every session created before now is refused.
    ///
    /// # Errors
    ///
    /// When hashing or the update fails.
    pub async fn reset_password<C: ConnectionTrait>(
        mut self,
        db: &C,
        password: &str,
    ) -> ModelResult<Model> {
        self.password = ActiveValue::set(hash_password(password)?);
        self.reset_token = ActiveValue::set(None);
        self.reset_sent_at = ActiveValue::set(None);
        self.sessions_valid_after = ActiveValue::set(Some(Utc::now().into()));
        Ok(self.update(db).await?)
    }
}
