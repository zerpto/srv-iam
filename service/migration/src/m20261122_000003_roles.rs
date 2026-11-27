use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// Built-in roles have no `tenant_id`; a tenant's own roles carry it.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "roles",
            &[
                ("id", ColType::PkUuid),
                ("tenant_id", ColType::UuidNull),
                ("name", ColType::String),
                ("role_type", ColType::String),
                ("description", ColType::TextNull),
                ("permissions", ColType::JsonBinary),
            ],
            &[],
        )
        .await?;
        m.get_connection()
            .execute_unprepared(
                "ALTER TABLE roles ADD CONSTRAINT fk_roles_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "roles").await
    }
}
