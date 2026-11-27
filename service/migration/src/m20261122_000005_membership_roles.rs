use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        create_table(
            m,
            "membership_roles",
            &[
                ("id", ColType::PkUuid),
                ("membership_id", ColType::Uuid),
                ("role_id", ColType::Uuid),
            ],
            &[],
        )
        .await?;
        let db = m.get_connection();
        db.execute_unprepared(
            "ALTER TABLE membership_roles ADD CONSTRAINT fk_membership_roles_membership FOREIGN KEY (membership_id) REFERENCES memberships (id) ON DELETE CASCADE",
        )
        .await?;
        db.execute_unprepared(
            "ALTER TABLE membership_roles ADD CONSTRAINT fk_membership_roles_role FOREIGN KEY (role_id) REFERENCES roles (id) ON DELETE CASCADE",
        )
        .await?;
        db.execute_unprepared(
            "CREATE UNIQUE INDEX idx_membership_roles_pair ON membership_roles (membership_id, role_id)",
        )
        .await?;
        Ok(())
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        drop_table(m, "membership_roles").await
    }
}
