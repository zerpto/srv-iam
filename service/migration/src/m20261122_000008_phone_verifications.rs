use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// The code a user was last sent to prove a phone number. Number and code are
/// stored only as hashes.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "phone_verifications",
            &[
                ("id", ColType::PkUuid),
                ("user_id", ColType::UuidUniq),
                ("phone_hash", ColType::String),
                ("channel", ColType::String),
                ("code_hash", ColType::String),
                ("expires_at", ColType::TimestampWithTimeZone),
                ("attempts", ColType::Integer),
                ("window_started_at", ColType::TimestampWithTimeZone),
                ("sends_in_window", ColType::Integer),
            ],
            &[],
        )
        .await?;
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE phone_verifications ADD CONSTRAINT fk_phone_verifications_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE",
        )
        .await?;
        db.execute_unprepared(
            "CREATE INDEX idx_phone_verifications_phone_hash ON phone_verifications (phone_hash)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "phone_verifications").await
    }
}
