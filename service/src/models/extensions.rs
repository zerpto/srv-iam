//! The extension registry: what the platform operator reviews.
//!
//! Every change to the registry or to what a tenant enabled holds one database
//! lock (see [`lock`]), so a dependency cannot be removed while another call
//! starts relying on it.

pub use super::_entities::extensions::{ActiveModel, Column, Entity, Model};
use std::collections::{BTreeMap, HashSet};

use loco_rs::prelude::*;
use sea_orm::TransactionTrait;
use semver::Version;
use uuid::Uuid;
use zerpto_core::http::ids::ApiIds;

use super::tenant_extensions;
use crate::{
    extensions::{
        manifest::Manifest,
        manifest_check::{self, ManifestError, CORE_KEYS},
    },
    http::ids::EXTENSION,
};

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

pub const PENDING_REVIEW: &str = "pending_review";
pub const APPROVED: &str = "approved";
pub const REJECTED: &str = "rejected";

/// `party` of every extension: only first-party ones exist.
pub const FIRST_PARTY: &str = "1st";

/// Why a change to the registry, or to what a tenant enabled, was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum ExtensionError {
    NotFound,
    Exists,
    /// The key belongs to a core extension, which cannot be registered.
    CoreKey,
    /// A core extension cannot be refreshed or disabled.
    IsCore,
    KeyMismatch,
    UnknownDependency(String),
    DependencyNotSatisfied(String),
    NothingToReview,
    /// Another enabled extension (named) depends on this one.
    Required(String),
    Manifest(ManifestError),
}

type Outcome<T> = ModelResult<Result<T, ExtensionError>>;

/// Serialises registry and enablement changes until the transaction ends.
///
/// # Errors
///
/// When the query fails.
pub async fn lock<C: ConnectionTrait>(txn: &C) -> ModelResult<()> {
    txn.execute_unprepared("SELECT pg_advisory_xact_lock(hashtext('iam.extensions'))")
        .await?;
    Ok(())
}

fn to_json(manifest: &Manifest) -> ModelResult<serde_json::Value> {
    serde_json::to_value(manifest).map_err(|err| ModelError::msg(&err.to_string()))
}

fn from_json(value: &serde_json::Value) -> ModelResult<Manifest> {
    serde_json::from_value(value.clone()).map_err(|err| ModelError::msg(&err.to_string()))
}

/// Every extension a manifest requires must be registered.
async fn dependencies_exist<C: ConnectionTrait>(db: &C, manifest: &Manifest) -> Outcome<()> {
    for key in manifest.requires.service_keys() {
        if Model::find_by_key(db, key).await?.is_none() {
            return Ok(Err(ExtensionError::UnknownDependency(key.to_string())));
        }
    }
    Ok(Ok(()))
}

impl Model {
    /// The API id: `ext-<uuid>`.
    #[must_use]
    pub fn api_id(&self) -> String {
        ApiIds::new(EXTENSION).encode(&self.id)
    }

    #[must_use]
    pub fn is_approved(&self) -> bool {
        self.status == APPROVED
    }

    /// The approved manifest, which is live.
    ///
    /// # Errors
    ///
    /// When the stored JSON is not a manifest.
    pub fn live_manifest(&self) -> ModelResult<Option<Manifest>> {
        self.manifest.as_ref().map(from_json).transpose()
    }

    /// The manifest waiting for review.
    ///
    /// # Errors
    ///
    /// When the stored JSON is not a manifest.
    pub fn waiting_manifest(&self) -> ModelResult<Option<Manifest>> {
        self.pending_manifest.as_ref().map(from_json).transpose()
    }

    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_key<C: ConnectionTrait>(db: &C, key: &str) -> ModelResult<Option<Self>> {
        Ok(Entity::find().filter(Column::Key.eq(key)).one(db).await?)
    }

    /// By API id; `None` for a malformed one.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_api_id<C: ConnectionTrait>(
        db: &C,
        api_id: &str,
    ) -> ModelResult<Option<Self>> {
        let Some(id) = ApiIds::new(EXTENSION).parse(api_id) else {
            return Ok(None);
        };
        Ok(Entity::find_by_id(id).one(db).await?)
    }

    /// Extensions, core ones first, then by key. `status` keeps only that one.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list<C: ConnectionTrait>(db: &C, status: Option<&str>) -> ModelResult<Vec<Self>> {
        let mut query = Entity::find();
        if let Some(status) = status {
            query = query.filter(Column::Status.eq(status));
        }
        let mut rows = query.all(db).await?;
        rows.sort_by(|a, b| (!a.core, &a.key).cmp(&(!b.core, &b.key)));
        Ok(rows)
    }

    /// Adds the core extensions, and updates those whose manifest or address
    /// changed. `addresses` says where each one's manifest is served, by key,
    /// and is stored as `manifest_url` (`NULL` for a key it lacks). A core
    /// extension with an address and no manifest here, because it waits for a
    /// fetch, gets only its `manifest_url` updated.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn install_core(
        db: &DatabaseConnection,
        manifests: &[Manifest],
        addresses: &BTreeMap<String, String>,
    ) -> ModelResult<()> {
        let txn = db.begin().await?;
        lock(&txn).await?;
        for manifest in manifests {
            let json = to_json(manifest)?;
            let address = addresses.get(&manifest.key);
            let existing = Self::find_by_key(&txn, &manifest.key).await?;
            if existing.as_ref().is_some_and(|row| {
                row.core
                    && row.is_approved()
                    && row.manifest.as_ref() == Some(&json)
                    && row.manifest_url.as_ref() == address
            }) {
                continue;
            }
            let insert = existing.is_none();
            let mut row = existing.map_or_else(
                || ActiveModel {
                    key: ActiveValue::set(manifest.key.clone()),
                    ..Default::default()
                },
                IntoActiveModel::into_active_model,
            );
            row.name = ActiveValue::set(manifest.name.clone());
            row.description = ActiveValue::set(manifest.description.clone());
            row.version = ActiveValue::set(manifest.version.clone());
            row.core = ActiveValue::set(true);
            row.status = ActiveValue::set(APPROVED.to_string());
            row.manifest_url = ActiveValue::set(address.cloned());
            row.manifest = ActiveValue::set(Some(json));
            row.pending_manifest = ActiveValue::set(None);
            row.rejection_reason = ActiveValue::set(None);
            if insert {
                row.insert(&txn).await?;
            } else {
                row.update(&txn).await?;
            }
        }
        for (key, address) in addresses {
            if manifests.iter().any(|manifest| &manifest.key == key) {
                continue;
            }
            let Some(row) = Self::find_by_key(&txn, key).await? else {
                continue;
            };
            if !row.core || row.manifest_url.as_ref() == Some(address) {
                continue;
            }
            let mut row = row.into_active_model();
            row.manifest_url = ActiveValue::set(Some(address.clone()));
            row.update(&txn).await?;
        }
        txn.commit().await?;
        Ok(())
    }

    /// Registers a fetched, checked manifest to wait for review.
    ///
    /// # Errors
    ///
    /// `CoreKey`, `Exists` or `UnknownDependency`.
    pub async fn register(
        db: &DatabaseConnection,
        manifest_url: &str,
        manifest: &Manifest,
    ) -> Outcome<Self> {
        if CORE_KEYS.contains(&manifest.key.as_str()) {
            return Ok(Err(ExtensionError::CoreKey));
        }
        let txn = db.begin().await?;
        lock(&txn).await?;
        if Self::find_by_key(&txn, &manifest.key).await?.is_some() {
            return Ok(Err(ExtensionError::Exists));
        }
        if let Err(err) = dependencies_exist(&txn, manifest).await? {
            return Ok(Err(err));
        }
        let row = ActiveModel {
            key: ActiveValue::set(manifest.key.clone()),
            name: ActiveValue::set(manifest.name.clone()),
            description: ActiveValue::set(manifest.description.clone()),
            version: ActiveValue::set(manifest.version.clone()),
            core: ActiveValue::set(false),
            status: ActiveValue::set(PENDING_REVIEW.to_string()),
            manifest_url: ActiveValue::set(Some(manifest_url.to_string())),
            manifest: ActiveValue::set(None),
            pending_manifest: ActiveValue::set(Some(to_json(manifest)?)),
            rejection_reason: ActiveValue::set(None),
            reviewed_at: ActiveValue::set(None),
            ..Default::default()
        }
        .insert(&txn)
        .await?;
        txn.commit().await?;
        Ok(Ok(row))
    }

    /// Stores a newly fetched manifest of this extension. One that equals the
    /// approved manifest leaves nothing to review.
    ///
    /// # Errors
    ///
    /// `NotFound`, `IsCore`, `KeyMismatch` or `UnknownDependency`.
    pub async fn refresh(db: &DatabaseConnection, id: Uuid, manifest: &Manifest) -> Outcome<Self> {
        let txn = db.begin().await?;
        lock(&txn).await?;
        let Some(current) = Entity::find_by_id(id).one(&txn).await? else {
            return Ok(Err(ExtensionError::NotFound));
        };
        if current.core {
            return Ok(Err(ExtensionError::IsCore));
        }
        if current.key != manifest.key {
            return Ok(Err(ExtensionError::KeyMismatch));
        }
        if let Err(err) = dependencies_exist(&txn, manifest).await? {
            return Ok(Err(err));
        }
        let live = current.live_manifest()?;
        let pending = if live.as_ref() == Some(manifest) {
            None
        } else {
            Some(to_json(manifest)?)
        };
        let mut row = current.into_active_model();
        // Until a manifest is approved, the submitted one names the extension.
        if live.is_none() {
            row.name = ActiveValue::set(manifest.name.clone());
            row.description = ActiveValue::set(manifest.description.clone());
            row.version = ActiveValue::set(manifest.version.clone());
            row.status = ActiveValue::set(PENDING_REVIEW.to_string());
        }
        row.pending_manifest = ActiveValue::set(pending);
        let row = row.update(&txn).await?;
        txn.commit().await?;
        Ok(Ok(row))
    }

    /// Makes the waiting manifest the live one. Tenants that already use the
    /// extension get any dependency it adds.
    ///
    /// # Errors
    ///
    /// `NotFound`, `NothingToReview`, `DependencyNotSatisfied` or `Manifest`.
    pub async fn approve(db: &DatabaseConnection, id: Uuid) -> Outcome<Self> {
        let txn = db.begin().await?;
        lock(&txn).await?;
        let Some(current) = Entity::find_by_id(id).one(&txn).await? else {
            return Ok(Err(ExtensionError::NotFound));
        };
        let Some(manifest) = current.waiting_manifest()? else {
            return Ok(Err(ExtensionError::NothingToReview));
        };
        if let Err(err) = manifest_check::check(&manifest) {
            return Ok(Err(ExtensionError::Manifest(err)));
        }
        for entry in &manifest.requires.services {
            let (key, range) = match manifest_check::requirement(entry) {
                Ok(requirement) => requirement,
                Err(err) => return Ok(Err(ExtensionError::Manifest(err))),
            };
            let satisfied = Self::find_by_key(&txn, key)
                .await?
                .filter(Self::is_approved)
                .is_some_and(|dependency| {
                    Version::parse(&dependency.version).is_ok_and(|v| range.matches(&v))
                });
            if !satisfied {
                return Ok(Err(ExtensionError::DependencyNotSatisfied(entry.clone())));
            }
        }
        let mut row = current.into_active_model();
        row.name = ActiveValue::set(manifest.name.clone());
        row.description = ActiveValue::set(manifest.description.clone());
        row.version = ActiveValue::set(manifest.version.clone());
        row.status = ActiveValue::set(APPROVED.to_string());
        row.manifest = ActiveValue::set(Some(to_json(&manifest)?));
        row.pending_manifest = ActiveValue::set(None);
        row.rejection_reason = ActiveValue::set(None);
        row.reviewed_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        let row = row.update(&txn).await?;

        let dependencies = row.dependencies(&txn).await?;
        for tenant_id in tenant_extensions::Model::tenants_using(&txn, row.id).await? {
            tenant_extensions::Model::add_missing(&txn, tenant_id, &dependencies).await?;
        }
        txn.commit().await?;
        Ok(Ok(row))
    }

    /// Refuses the waiting manifest. A new registration becomes `rejected`; an
    /// approved extension stays as it was.
    ///
    /// # Errors
    ///
    /// `NotFound` or `NothingToReview`.
    pub async fn reject(db: &DatabaseConnection, id: Uuid, reason: &str) -> Outcome<Self> {
        let txn = db.begin().await?;
        lock(&txn).await?;
        let Some(current) = Entity::find_by_id(id).one(&txn).await? else {
            return Ok(Err(ExtensionError::NotFound));
        };
        if current.pending_manifest.is_none() {
            return Ok(Err(ExtensionError::NothingToReview));
        }
        let never_approved = current.manifest.is_none();
        let mut row = current.into_active_model();
        if never_approved {
            row.status = ActiveValue::set(REJECTED.to_string());
        }
        row.pending_manifest = ActiveValue::set(None);
        row.rejection_reason = ActiveValue::set(Some(reason.to_string()));
        row.reviewed_at = ActiveValue::set(Some(chrono::Utc::now().into()));
        let row = row.update(&txn).await?;
        txn.commit().await?;
        Ok(Ok(row))
    }

    /// Every non-core extension this one needs, directly or through another,
    /// by its live manifest. The extension itself is not included.
    ///
    /// # Errors
    ///
    /// When a query fails, or a dependency is not approved.
    pub async fn dependencies<C: ConnectionTrait>(&self, db: &C) -> ModelResult<Vec<Self>> {
        let mut seen = HashSet::from([self.key.clone()]);
        let mut found = Vec::new();
        let mut queue = vec![self.clone()];
        while let Some(extension) = queue.pop() {
            let Some(manifest) = extension.live_manifest()? else {
                continue;
            };
            for key in manifest.requires.service_keys() {
                if !seen.insert(key.to_string()) {
                    continue;
                }
                let dependency = Self::find_by_key(db, key)
                    .await?
                    .filter(Self::is_approved)
                    .ok_or_else(|| ModelError::msg("a required extension is not approved"))?;
                queue.push(dependency.clone());
                found.push(dependency);
            }
        }
        Ok(found)
    }
}
