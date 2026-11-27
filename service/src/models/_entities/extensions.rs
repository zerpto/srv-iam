//! `SeaORM` Entity, matches `m20261126_000001_extensions`

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "extensions")]
pub struct Model {
    pub created_at: DateTimeWithTimeZone,
    pub updated_at: DateTimeWithTimeZone,
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Uuid,
    #[sea_orm(unique)]
    pub key: String,
    pub name: String,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    pub version: String,
    pub core: bool,
    pub status: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub manifest_url: Option<String>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub manifest: Option<Json>,
    #[sea_orm(column_type = "JsonBinary", nullable)]
    pub pending_manifest: Option<Json>,
    #[sea_orm(column_type = "Text", nullable)]
    pub rejection_reason: Option<String>,
    pub reviewed_at: Option<DateTimeWithTimeZone>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::tenant_extensions::Entity")]
    TenantExtensions,
}

impl Related<super::tenant_extensions::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::TenantExtensions.def()
    }
}
