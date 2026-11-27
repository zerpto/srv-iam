//! The permission catalogue: the keys that extensions declare in their
//! manifests, `{extension}.{resource}.{action}`.
//!
//! Roles and direct grants may use any key an approved extension declares. A
//! key only has effect while its extension is core or enabled for the tenant.

use loco_rs::prelude::*;
use uuid::Uuid;

use super::{
    extensions,
    plan::{self, Feature},
    roles::ALL_PERMISSIONS,
    tenant_extensions,
};
use crate::extensions::manifest::{Kind, ManifestPermission, MinimumPlan};

/// One permission of the catalogue.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Permission {
    pub key: String,
    /// Key of the extension that declares it.
    pub module: String,
    pub write: bool,
    pub paid: bool,
    pub description: String,
}

impl Permission {
    #[must_use]
    pub fn declared(module: &str, permission: ManifestPermission) -> Self {
        Self {
            key: permission.key,
            module: module.to_string(),
            write: permission.kind == Kind::Write,
            paid: permission.minimum_plan == MinimumPlan::Paid,
            description: permission.description,
        }
    }
}

/// The permissions of a set of extensions.
#[derive(Debug, Clone, Default)]
pub struct Catalogue(Vec<Permission>);

impl Catalogue {
    #[must_use]
    pub fn new(mut permissions: Vec<Permission>) -> Self {
        permissions.sort_by(|a, b| (&a.module, &a.key).cmp(&(&b.module, &b.key)));
        Self(permissions)
    }

    fn of(extensions: &[extensions::Model]) -> ModelResult<Self> {
        let mut permissions = Vec::new();
        for extension in extensions {
            let Some(manifest) = extension.live_manifest()? else {
                continue;
            };
            permissions.extend(
                manifest
                    .backend
                    .permissions
                    .into_iter()
                    .map(|permission| Permission::declared(&extension.key, permission)),
            );
        }
        Ok(Self::new(permissions))
    }

    /// Every key an approved extension declares, enabled or not.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn declared<C: ConnectionTrait>(db: &C) -> ModelResult<Self> {
        Self::of(&extensions::Model::list(db, Some(extensions::APPROVED)).await?)
    }

    /// The keys in effect for a tenant: those of the core extensions and of
    /// the ones it enabled. With no tenant, only the core ones.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn enabled<C: ConnectionTrait>(db: &C, tenant_id: Option<Uuid>) -> ModelResult<Self> {
        Self::of(&tenant_extensions::Model::active(db, tenant_id).await?)
    }

    #[must_use]
    pub fn permissions(&self) -> &[Permission] {
        &self.0
    }

    #[must_use]
    pub fn is_known(&self, key: &str) -> bool {
        self.0.iter().any(|permission| permission.key == key)
    }

    /// The keys of `held` that are in this catalogue, sorted. The
    /// all-permissions key `*` stands for every one of them.
    #[must_use]
    pub fn narrow(&self, held: &[String]) -> Vec<String> {
        let all = held.iter().any(|key| key == ALL_PERMISSIONS);
        let mut keys: Vec<String> = self
            .0
            .iter()
            .filter(|permission| all || held.contains(&permission.key))
            .map(|permission| permission.key.clone())
            .collect();
        keys.sort();
        keys.dedup();
        keys
    }

    /// The features the plan rules go by: what the manifests declare, replaced
    /// by the plan provider's entry (`overrides`) for any key it lists.
    #[must_use]
    pub fn features(&self, overrides: Vec<Feature>) -> Vec<Feature> {
        let mut features: Vec<Feature> = self
            .0
            .iter()
            .filter(|permission| !overrides.iter().any(|f| f.key == permission.key))
            .map(|permission| Feature {
                key: permission.key.clone(),
                name: permission.description.clone(),
                paid: permission.paid,
                write: permission.write,
            })
            .collect();
        features.extend(overrides);
        features
    }

    /// What a person who holds `held` may really do: only keys of this
    /// catalogue, without the paid ones when the plan is `locked` and without
    /// writes when `read_only`.
    #[must_use]
    pub fn allowed(
        &self,
        held: &[String],
        overrides: Vec<Feature>,
        locked: bool,
        read_only: bool,
    ) -> Vec<String> {
        plan::allowed_permissions(
            &self.features(overrides),
            &self.narrow(held),
            locked,
            read_only,
        )
    }
}
