use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// An emailed invitation to join a tenant. Only the hash of the token is kept.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "invitations",
            &[
                ("id", ColType::PkUuid),
                ("tenant_id", ColType::Uuid),
                ("email", ColType::String),
                ("name", ColType::StringNull),
                ("token_hash", ColType::StringUniq),
                // JSON array of role uuids chosen in the invitation.
                ("role_ids", ColType::JsonBinary),
                ("expires_at", ColType::TimestampWithTimeZone),
                ("accepted_at", ColType::TimestampWithTimeZoneNull),
            ],
            &[],
        )
        .await?;
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE invitations ADD CONSTRAINT fk_invitations_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "invitations").await
    }
}
