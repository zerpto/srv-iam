use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        add_column(m, "tenants", "legal_name", ColType::StringNull).await?;
        add_column(m, "tenants", "tax_id", ColType::StringNull).await?;
        add_column(m, "tenants", "base_currency", ColType::StringNull).await?;
        add_column(m, "tenants", "timezone", ColType::StringNull).await?;
        add_column(
            m,
            "tenants",
            "fiscal_year_start_month",
            ColType::BigIntegerNull,
        )
        .await?;
        add_column(m, "tenants", "completeness", ColType::StringNull).await?;
        add_column(m, "tenants", "region_id", ColType::UuidNull).await?;
        add_column(m, "tenants", "line1", ColType::StringNull).await?;
        add_column(m, "tenants", "line2", ColType::StringNull).await?;
        add_column(m, "tenants", "country", ColType::StringNull).await?;
        add_column(m, "tenants", "province", ColType::StringNull).await?;
        add_column(m, "tenants", "district", ColType::StringNull).await?;
        add_column(m, "tenants", "sub_district", ColType::StringNull).await?;
        add_column(m, "tenants", "village", ColType::StringNull).await?;
        add_column(m, "tenants", "postal_code", ColType::StringNull).await?;
        // Nullable but unique: Postgres allows many tenants without a tax id.
        m.get_connection()
            .execute_unprepared(
                "CREATE UNIQUE INDEX idx_tenants_tax_id ON tenants (tax_id) WHERE tax_id IS NOT NULL",
            )
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared("DROP INDEX IF EXISTS idx_tenants_tax_id")
            .await?;
        remove_column(m, "tenants", "legal_name").await?;
        remove_column(m, "tenants", "tax_id").await?;
        remove_column(m, "tenants", "base_currency").await?;
        remove_column(m, "tenants", "timezone").await?;
        remove_column(m, "tenants", "fiscal_year_start_month").await?;
        remove_column(m, "tenants", "completeness").await?;
        remove_column(m, "tenants", "region_id").await?;
        remove_column(m, "tenants", "line1").await?;
        remove_column(m, "tenants", "line2").await?;
        remove_column(m, "tenants", "country").await?;
        remove_column(m, "tenants", "province").await?;
        remove_column(m, "tenants", "district").await?;
        remove_column(m, "tenants", "sub_district").await?;
        remove_column(m, "tenants", "village").await?;
        remove_column(m, "tenants", "postal_code").await?;
        Ok(())
    }
}
