use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// People who can sign in. Email is unique across all tenants.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "users",
            &[
                ("id", ColType::PkUuid),
                ("email", ColType::StringUniq),
                ("name", ColType::String),
                ("password", ColType::String),
                ("email_verification_token", ColType::StringNull),
                (
                    "email_verification_sent_at",
                    ColType::TimestampWithTimeZoneNull,
                ),
                ("email_verified_at", ColType::TimestampWithTimeZoneNull),
                ("reset_token", ColType::StringNull),
                ("reset_sent_at", ColType::TimestampWithTimeZoneNull),
                // Sessions created before this moment are refused (password change).
                ("sessions_valid_after", ColType::TimestampWithTimeZoneNull),
                // Hash of the verified phone number; one trial per number.
                ("phone_hash", ColType::StringNull),
                ("phone_verified_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        let db = m.get_connection();
        db.execute_unprepared(
            "CREATE INDEX idx_users_email_verification_token ON users (email_verification_token)",
        )
        .await?;
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_users_phone_hash ON users (phone_hash) WHERE phone_hash IS NOT NULL",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "users").await
    }
}
