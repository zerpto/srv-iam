//! Proving that a person owns a phone number, with a 6-digit code sent by
//! `WhatsApp` or SMS. A verified number is what lets a person start their one
//! free trial, so a number is stored only as a hash.

pub use super::_entities::phone_verifications::{ActiveModel, Column, Entity, Model};
use chrono::{DateTime, Duration, Utc};
use loco_rs::prelude::*;
use sea_orm::TransactionTrait;
use uuid::Uuid;

use super::{digest::sha256_hex, users};
use crate::settings::PhoneSettings;

pub const WHATSAPP: &str = "whatsapp";
pub const SMS: &str = "sms";

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

/// Why a code was not sent.
#[derive(Debug, PartialEq, Eq)]
pub enum SendError {
    InvalidPhone,
    TooManyCodes,
}

/// Why a code was not accepted.
#[derive(Debug, PartialEq, Eq)]
pub enum VerifyError {
    InvalidCode,
    CodeExpired,
    TooManyAttempts,
}

/// A phone number in E.164 form (`+6281234567890`), spaces and dashes
/// removed; `None` when it is not one.
#[must_use]
pub fn normalize(phone: &str) -> Option<String> {
    let cleaned: String = phone
        .chars()
        .filter(|c| !matches!(c, ' ' | '-' | '(' | ')'))
        .collect();
    let digits = cleaned.strip_prefix('+')?;
    let valid = (8..=15).contains(&digits.len())
        && digits.chars().all(|c| c.is_ascii_digit())
        && !digits.starts_with('0');
    valid.then_some(cleaned)
}

/// What is stored for a number. The secret keeps the hashes from being
/// guessed by trying every phone number.
#[must_use]
pub fn hash_phone(secret: &str, phone: &str) -> String {
    sha256_hex(&format!("{secret}:phone:{phone}"))
}

fn hash_code(secret: &str, phone: &str, code: &str) -> String {
    sha256_hex(&format!("{secret}:code:{phone}:{code}"))
}

/// Six random digits.
#[must_use]
pub fn generate_code() -> String {
    let bytes = *Uuid::new_v4().as_bytes();
    let number = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) % 1_000_000;
    format!("{number:06}")
}

impl Model {
    /// Issues a new code for `user_id`, replacing any earlier one, and returns
    /// it for the caller to send. Asking is limited per person and per number.
    ///
    /// # Errors
    ///
    /// `Ok(Err(..))` says why no code was issued; the outer error is a failed
    /// query.
    pub async fn issue<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        phone: &str,
        channel: &str,
        secret: &str,
        settings: &PhoneSettings,
        now: DateTime<Utc>,
    ) -> ModelResult<Result<String, SendError>> {
        let Some(phone) = normalize(phone) else {
            return Ok(Err(SendError::InvalidPhone));
        };
        let phone_hash = hash_phone(secret, &phone);
        let window = now - Duration::hours(1);

        // Other people asking for the same number recently.
        let others: i32 = Entity::find()
            .filter(Column::PhoneHash.eq(phone_hash.as_str()))
            .filter(Column::UserId.ne(user_id))
            .filter(Column::WindowStartedAt.gt(window))
            .all(db)
            .await?
            .iter()
            .map(|row| row.sends_in_window)
            .sum();
        if others >= settings.max_sends_per_hour {
            return Ok(Err(SendError::TooManyCodes));
        }

        let existing = Entity::find()
            .filter(Column::UserId.eq(user_id))
            .one(db)
            .await?;
        let (window_started_at, sends_in_window) = match &existing {
            Some(row) if row.window_started_at.with_timezone(&Utc) > window => {
                if row.sends_in_window >= settings.max_sends_per_hour {
                    return Ok(Err(SendError::TooManyCodes));
                }
                (
                    row.window_started_at.with_timezone(&Utc),
                    row.sends_in_window + 1,
                )
            }
            _ => (now, 1),
        };

        let code = generate_code();
        let is_new = existing.is_none();
        let mut row = existing.map_or_else(
            || ActiveModel {
                user_id: ActiveValue::set(user_id),
                ..Default::default()
            },
            IntoActiveModel::into_active_model,
        );
        row.phone_hash = ActiveValue::set(phone_hash);
        row.channel = ActiveValue::set(channel.to_string());
        row.code_hash = ActiveValue::set(hash_code(secret, &phone, &code));
        row.expires_at =
            ActiveValue::set((now + Duration::minutes(settings.code_ttl_minutes)).into());
        row.attempts = ActiveValue::set(0);
        row.window_started_at = ActiveValue::set(window_started_at.into());
        row.sends_in_window = ActiveValue::set(sends_in_window);
        if is_new {
            row.insert(db).await?;
        } else {
            row.update(db).await?;
        }
        Ok(Ok(code))
    }

    /// Checks `code` for `user`. On success the number is verified for them.
    /// Wrong guesses are counted; after too many a new code is needed.
    ///
    /// # Errors
    ///
    /// `Ok(Err(..))` says why the code was refused; the outer error is a
    /// failed query.
    pub async fn verify(
        db: &DatabaseConnection,
        user: &users::Model,
        phone: &str,
        code: &str,
        secret: &str,
        settings: &PhoneSettings,
        now: DateTime<Utc>,
    ) -> ModelResult<Result<(), VerifyError>> {
        let Some(phone) = normalize(phone) else {
            return Ok(Err(VerifyError::InvalidCode));
        };
        let Some(row) = Entity::find()
            .filter(Column::UserId.eq(user.id))
            .one(db)
            .await?
        else {
            return Ok(Err(VerifyError::InvalidCode));
        };
        let phone_hash = hash_phone(secret, &phone);
        if row.phone_hash != phone_hash {
            return Ok(Err(VerifyError::InvalidCode));
        }
        if row.expires_at.with_timezone(&Utc) <= now {
            return Ok(Err(VerifyError::CodeExpired));
        }
        if row.attempts >= settings.max_attempts {
            return Ok(Err(VerifyError::TooManyAttempts));
        }
        if row.code_hash != hash_code(secret, &phone, code.trim()) {
            let attempts = row.attempts + 1;
            let mut wrong = row.into_active_model();
            wrong.attempts = ActiveValue::set(attempts);
            wrong.update(db).await?;
            return Ok(Err(VerifyError::InvalidCode));
        }

        let txn = db.begin().await?;
        let mut verified = user.clone().into_active_model();
        verified.phone_hash = ActiveValue::set(Some(phone_hash));
        verified.phone_verified_at = ActiveValue::set(Some(now.into()));
        match verified.update(&txn).await {
            Ok(_) => {}
            // Another account already proved this number: one trial per number.
            Err(err)
                if matches!(
                    err.sql_err(),
                    Some(sea_orm::SqlErr::UniqueConstraintViolation(_))
                ) =>
            {
                return Ok(Err(VerifyError::InvalidCode));
            }
            Err(err) => return Err(err.into()),
        }
        row.delete(&txn).await?;
        txn.commit().await?;
        Ok(Ok(()))
    }
}
