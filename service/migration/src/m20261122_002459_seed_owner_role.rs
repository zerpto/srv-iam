use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

/// The built-in `owner` role every tenant creator is given. It belongs to no
/// tenant (`tenant_id` is null) and carries `*`, which grants everything. The
/// id is fixed so it is the same in every environment.
const OWNER_ROLE_ID: &str = "01935c8e-0000-7000-8000-000000000001";

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(&format!(
                "INSERT INTO roles (id, tenant_id, name, role_type, description, permissions) \
                 VALUES ('{OWNER_ROLE_ID}', NULL, 'owner', 'system', 'Full access to the tenant.', '[\"*\"]'::jsonb) \
                 ON CONFLICT (id) DO NOTHING"
            ))
            .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.get_connection()
            .execute_unprepared(&format!("DELETE FROM roles WHERE id = '{OWNER_ROLE_ID}'"))
            .await?;
        Ok(())
    }
}
