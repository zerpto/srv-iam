use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// A user's place in a tenant. An accepted invitation keeps its id as the
/// membership id, so the `mem-` id the API showed does not change.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "memberships",
            &[
                ("id", ColType::PkUuid),
                ("user_id", ColType::Uuid),
                ("tenant_id", ColType::Uuid),
                ("last_used_at", ColType::TimestampWithTimeZoneNull),
                ("deactivated_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE memberships ADD CONSTRAINT fk_memberships_user FOREIGN KEY (user_id) REFERENCES users (id) ON DELETE CASCADE",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE memberships ADD CONSTRAINT fk_memberships_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE",
        )
        .await?;
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_memberships_user_tenant ON memberships (user_id, tenant_id)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "memberships").await
    }
}
