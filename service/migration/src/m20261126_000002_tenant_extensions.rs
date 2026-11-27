use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// The non-core extensions a tenant has enabled: a row means enabled.
#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "tenant_extensions",
            &[
                ("id", ColType::PkUuid),
                ("tenant_id", ColType::Uuid),
                ("extension_id", ColType::Uuid),
            ],
            &[],
        )
        .await?;
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE tenant_extensions ADD CONSTRAINT fk_tenant_extensions_tenant FOREIGN KEY (tenant_id) REFERENCES tenants (id) ON DELETE CASCADE",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE tenant_extensions ADD CONSTRAINT fk_tenant_extensions_extension FOREIGN KEY (extension_id) REFERENCES extensions (id) ON DELETE CASCADE",
        )
        .await?;
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_tenant_extensions_pair ON tenant_extensions (tenant_id, extension_id)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "tenant_extensions").await
    }
}
