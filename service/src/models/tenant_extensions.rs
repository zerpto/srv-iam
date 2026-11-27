//! Which extensions a tenant uses. Core extensions are always on and have no
//! rows; a row means a non-core extension is enabled.

pub use super::_entities::tenant_extensions::{ActiveModel, Column, Entity, Model};
use std::collections::HashSet;

use loco_rs::prelude::*;
use sea_orm::TransactionTrait;
use uuid::Uuid;

use super::extensions::{self, ExtensionError};

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert && self.id.is_not_set() {
            self.id = ActiveValue::Set(Uuid::now_v7());
        }
        if !insert && self.updated_at.is_unchanged() {
            self.updated_at = ActiveValue::Set(chrono::Utc::now().into());
        }
        Ok(self)
    }
}

type Outcome<T> = ModelResult<Result<T, ExtensionError>>;

impl Model {
    /// The ids of the non-core extensions the tenant enabled.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn enabled_ids<C: ConnectionTrait>(
        db: &C,
        tenant_id: Uuid,
    ) -> ModelResult<HashSet<Uuid>> {
        Ok(Entity::find()
            .filter(Column::TenantId.eq(tenant_id))
            .all(db)
            .await?
            .into_iter()
            .map(|row| row.extension_id)
            .collect())
    }

    /// The core extensions and the ones the tenant enabled, all approved. With
    /// no tenant, only the core ones.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn active<C: ConnectionTrait>(
        db: &C,
        tenant_id: Option<Uuid>,
    ) -> ModelResult<Vec<extensions::Model>> {
        let enabled = match tenant_id {
            Some(tenant_id) => Self::enabled_ids(db, tenant_id).await?,
            None => HashSet::new(),
        };
        Ok(extensions::Model::list(db, Some(extensions::APPROVED))
            .await?
            .into_iter()
            .filter(|extension| extension.core || enabled.contains(&extension.id))
            .collect())
    }

    pub(super) async fn tenants_using<C: ConnectionTrait>(
        db: &C,
        extension_id: Uuid,
    ) -> ModelResult<Vec<Uuid>> {
        Ok(Entity::find()
            .filter(Column::ExtensionId.eq(extension_id))
            .all(db)
            .await?
            .into_iter()
            .map(|row| row.tenant_id)
            .collect())
    }

    /// Enables those of `wanted` the tenant lacks, and returns their keys.
    pub(super) async fn add_missing<C: ConnectionTrait>(
        db: &C,
        tenant_id: Uuid,
        wanted: &[extensions::Model],
    ) -> ModelResult<Vec<String>> {
        let enabled = Self::enabled_ids(db, tenant_id).await?;
        let mut added = Vec::new();
        for extension in wanted {
            if extension.core || enabled.contains(&extension.id) {
                continue;
            }
            ActiveModel {
                tenant_id: ActiveValue::set(tenant_id),
                extension_id: ActiveValue::set(extension.id),
                ..Default::default()
            }
            .insert(db)
            .await?;
            added.push(extension.key.clone());
        }
        added.sort();
        Ok(added)
    }

    /// Enables every approved extension for a tenant that is being created.
    /// Run it in the transaction that creates the tenant.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn enable_all_approved<C: ConnectionTrait>(
        txn: &C,
        tenant_id: Uuid,
    ) -> ModelResult<()> {
        extensions::lock(txn).await?;
        let approved = extensions::Model::list(txn, Some(extensions::APPROVED)).await?;
        Self::add_missing(txn, tenant_id, &approved).await?;
        Ok(())
    }

    /// Enables an approved extension and everything it depends on. Returns the
    /// extension and the keys of the dependencies switched on by this call.
    ///
    /// # Errors
    ///
    /// `NotFound` when it is not an approved extension.
    pub async fn enable(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        api_id: &str,
    ) -> Outcome<(extensions::Model, Vec<String>)> {
        let txn = db.begin().await?;
        extensions::lock(&txn).await?;
        let Some(extension) = extensions::Model::find_by_api_id(&txn, api_id)
            .await?
            .filter(extensions::Model::is_approved)
        else {
            return Ok(Err(ExtensionError::NotFound));
        };
        let dependencies = extension.dependencies(&txn).await?;
        let also_enabled = Self::add_missing(&txn, tenant_id, &dependencies).await?;
        Self::add_missing(&txn, tenant_id, std::slice::from_ref(&extension)).await?;
        txn.commit().await?;
        Ok(Ok((extension, also_enabled)))
    }

    /// Disables an extension. Its data and the roles' permission keys are kept.
    ///
    /// # Errors
    ///
    /// `NotFound`, `IsCore`, or `Required` while another enabled extension
    /// depends on it.
    pub async fn disable(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        api_id: &str,
    ) -> Outcome<extensions::Model> {
        let txn = db.begin().await?;
        extensions::lock(&txn).await?;
        let Some(extension) = extensions::Model::find_by_api_id(&txn, api_id)
            .await?
            .filter(extensions::Model::is_approved)
        else {
            return Ok(Err(ExtensionError::NotFound));
        };
        if extension.core {
            return Ok(Err(ExtensionError::IsCore));
        }
        for other in Self::active(&txn, Some(tenant_id)).await? {
            if other.id == extension.id {
                continue;
            }
            let depends = other.live_manifest()?.is_some_and(|manifest| {
                manifest
                    .requires
                    .service_keys()
                    .contains(&extension.key.as_str())
            });
            if depends {
                return Ok(Err(ExtensionError::Required(other.key)));
            }
        }
        Entity::delete_many()
            .filter(Column::TenantId.eq(tenant_id))
            .filter(Column::ExtensionId.eq(extension.id))
            .exec(&txn)
            .await?;
        txn.commit().await?;
        Ok(Ok(extension))
    }
}
