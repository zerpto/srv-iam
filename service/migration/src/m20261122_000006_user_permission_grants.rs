use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// A permission given to one member directly, with an optional expiry.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "user_permission_grants",
            &[
                ("id", ColType::PkUuid),
                ("membership_id", ColType::Uuid),
                ("permission", ColType::String),
                ("expires_at", ColType::TimestampWithTimeZoneNull),
                ("granted_by_user_id", ColType::UuidNull),
            ],
            &[],
        )
        .await?;
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE user_permission_grants ADD CONSTRAINT fk_grants_membership FOREIGN KEY (membership_id) REFERENCES memberships (id) ON DELETE CASCADE",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE user_permission_grants ADD CONSTRAINT fk_grants_granted_by FOREIGN KEY (granted_by_user_id) REFERENCES users (id) ON DELETE SET NULL",
        )
        .await?;
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_grants_membership_permission ON user_permission_grants (membership_id, permission)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "user_permission_grants").await
    }
}
