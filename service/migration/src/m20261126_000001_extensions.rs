use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// The platform's extension registry. `manifest` is the approved one and
/// `pending_manifest` the one waiting for review.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "extensions",
            &[
                ("id", ColType::PkUuid),
                ("key", ColType::String),
                ("name", ColType::String),
                ("description", ColType::Text),
                ("version", ColType::String),
                ("core", ColType::Boolean),
                ("status", ColType::String),
                ("manifest_url", ColType::TextNull),
                ("manifest", ColType::JsonBinaryNull),
                ("pending_manifest", ColType::JsonBinaryNull),
                ("rejection_reason", ColType::TextNull),
                ("reviewed_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        m.get_connection()
            .execute_unprepared("CREATE UNIQUE INDEX idx_extensions_key ON extensions (key)")
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "extensions").await
    }
}
