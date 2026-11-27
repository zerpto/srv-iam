pub use super::_entities::roles::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use sea_orm::{Condition, PaginatorTrait};
use uuid::Uuid;
use zerpto_core::http::ids::ApiIds;

use super::{
    _entities::{invitations, membership_roles, memberships},
    permissions::Catalogue,
};
use crate::http::ids::ROLE;

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

/// Name of the built-in role given to whoever creates a tenant.
pub const OWNER: &str = "owner";

/// The permission key that stands for every permission.
pub const ALL_PERMISSIONS: &str = "*";

/// `roles.role_type` of the built-in roles, which belong to no tenant.
pub const SYSTEM: &str = "system";
/// `roles.role_type` of a role a tenant made.
pub const CUSTOM: &str = "custom";

/// Why a role could not be created, changed or deleted.
#[derive(Debug, PartialEq, Eq)]
pub enum RoleError {
    NotFound,
    /// Another role the tenant can see already has the name.
    NameTaken,
    /// A permission key is not in the catalogue.
    UnknownPermission(String),
    /// The caller does not hold a permission they tried to give.
    CannotGrant(String),
    /// Built-in roles cannot be changed or deleted.
    IsSystem,
    /// Someone still holds the role.
    InUse,
    /// A new role needs permissions or a role to copy them from.
    PermissionsRequired,
}

/// A role with the number of the tenant's users who hold it.
#[derive(Debug, Clone)]
pub struct Detail {
    pub role: Model,
    pub user_count: u64,
}

/// What `PATCH /v1/iam/roles/{id}` changes; `None` leaves the field alone.
#[derive(Debug, Default)]
pub struct Changes {
    pub name: Option<String>,
    /// `Some(None)` clears the description.
    pub description: Option<Option<String>>,
    pub permissions: Option<Vec<String>>,
}

type Outcome<T> = ModelResult<Result<T, RoleError>>;

fn visible_to(tenant_id: Uuid) -> Condition {
    Condition::any()
        .add(Column::TenantId.is_null())
        .add(Column::TenantId.eq(tenant_id))
}

fn tidy(mut keys: Vec<String>) -> Vec<String> {
    keys.sort();
    keys.dedup();
    keys
}

/// The union of the permission keys of `roles`, sorted and de-duplicated.
#[must_use]
pub fn effective_permissions(roles: &[Model]) -> Vec<String> {
    tidy(roles.iter().flat_map(Model::permission_keys).collect())
}

/// Whether `permissions` (the effective set of a session) grants `key`:
/// either the key itself or the all-permissions key `*`.
#[must_use]
pub fn grants(permissions: &[String], key: &str) -> bool {
    permissions
        .iter()
        .any(|granted| granted == ALL_PERMISSIONS || granted == key)
}

/// Checks that every key is in the catalogue (declared by an approved
/// extension) and held by the caller. Keys in `already` (kept from the role
/// being edited) need only exist.
fn check_permissions(
    catalogue: &Catalogue,
    keys: &[String],
    caller: &[String],
    already: &[String],
) -> Result<(), RoleError> {
    for key in keys {
        if !catalogue.is_known(key) {
            return Err(RoleError::UnknownPermission(key.clone()));
        }
        if !already.contains(key) && !grants(caller, key) {
            return Err(RoleError::CannotGrant(key.clone()));
        }
    }
    Ok(())
}

impl Model {
    /// A built-in role (one that belongs to no tenant) by name.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_system<C: ConnectionTrait>(db: &C, name: &str) -> ModelResult<Option<Self>> {
        Ok(Entity::find()
            .filter(Column::TenantId.is_null())
            .filter(Column::Name.eq(name))
            .one(db)
            .await?)
    }

    /// The API id of the built-in owner role, which every tenant shares.
    ///
    /// # Errors
    ///
    /// When the query fails or the role is missing (the migrations seed it).
    pub async fn owner_api_id<C: ConnectionTrait>(db: &C) -> ModelResult<String> {
        Self::find_system(db, OWNER)
            .await?
            .map(|role| role.api_id())
            .ok_or_else(|| ModelError::msg("the built-in owner role is missing"))
    }

    /// The API id: `rol-<uuid>`.
    #[must_use]
    pub fn api_id(&self) -> String {
        ApiIds::new(ROLE).encode(&self.id)
    }

    /// The permission keys this role grants.
    #[must_use]
    pub fn permission_keys(&self) -> Vec<String> {
        serde_json::from_value(self.permissions.clone()).unwrap_or_default()
    }

    #[must_use]
    pub fn is_system(&self) -> bool {
        self.role_type == SYSTEM
    }

    /// A role the tenant can see (built-in or its own) by API id.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_visible<C: ConnectionTrait>(
        db: &C,
        tenant_id: Uuid,
        api_id: &str,
    ) -> ModelResult<Option<Self>> {
        let Some(id) = ApiIds::new(ROLE).parse(api_id) else {
            return Ok(None);
        };
        Ok(Entity::find()
            .filter(Column::Id.eq(id))
            .filter(visible_to(tenant_id))
            .one(db)
            .await?)
    }

    /// The built-in roles and the tenant's own, system roles first, then by
    /// name. `role_type` keeps only that type.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn list_visible<C: ConnectionTrait>(
        db: &C,
        tenant_id: Uuid,
        role_type: Option<&str>,
        paging: &loco_rs::model::query::PaginationQuery,
    ) -> ModelResult<(Vec<Detail>, u64)> {
        let mut query = Entity::find().filter(visible_to(tenant_id));
        if let Some(role_type) = role_type {
            query = query.filter(Column::RoleType.eq(role_type));
        }
        let mut all = query.all(db).await?;
        all.sort_by(|a, b| {
            (!a.is_system(), a.name.to_lowercase()).cmp(&(!b.is_system(), b.name.to_lowercase()))
        });
        let total = all.len() as u64;
        let skip = usize::try_from((paging.page - 1).saturating_mul(paging.page_size))
            .unwrap_or(usize::MAX);
        let page: Vec<Self> = all
            .into_iter()
            .skip(skip)
            .take(usize::try_from(paging.page_size).unwrap_or(usize::MAX))
            .collect();
        Ok((Self::with_counts(db, tenant_id, page).await?, total))
    }

    /// Attaches each role's `user_count` within the tenant.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn with_counts<C: ConnectionTrait>(
        db: &C,
        tenant_id: Uuid,
        roles: Vec<Self>,
    ) -> ModelResult<Vec<Detail>> {
        let members: Vec<Uuid> = memberships::Entity::find()
            .filter(memberships::Column::TenantId.eq(tenant_id))
            .all(db)
            .await?
            .into_iter()
            .map(|membership| membership.id)
            .collect();
        let links = membership_roles::Entity::find()
            .filter(membership_roles::Column::MembershipId.is_in(members))
            .filter(membership_roles::Column::RoleId.is_in(roles.iter().map(|role| role.id)))
            .all(db)
            .await?;
        Ok(roles
            .into_iter()
            .map(|role| {
                let user_count = links.iter().filter(|link| link.role_id == role.id).count();
                Detail {
                    role,
                    user_count: user_count as u64,
                }
            })
            .collect())
    }

    /// One role with its user count.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn detail<C: ConnectionTrait>(self, db: &C, tenant_id: Uuid) -> ModelResult<Detail> {
        Ok(Self::with_counts(db, tenant_id, vec![self])
            .await?
            .remove(0))
    }

    async fn name_taken<C: ConnectionTrait>(
        db: &C,
        tenant_id: Uuid,
        name: &str,
        except: Option<Uuid>,
    ) -> ModelResult<bool> {
        let wanted = name.to_lowercase();
        Ok(Entity::find()
            .filter(visible_to(tenant_id))
            .all(db)
            .await?
            .iter()
            .any(|role| Some(role.id) != except && role.name.to_lowercase() == wanted))
    }

    /// Creates a custom role. With `clone_from`, its permissions are the
    /// starting point when `permissions` is not given.
    ///
    /// # Errors
    ///
    /// `NameTaken`, `UnknownPermission`, `CannotGrant`, `PermissionsRequired`,
    /// or `NotFound` when `clone_from` is not a role of the tenant.
    pub async fn create_custom(
        db: &DatabaseConnection,
        tenant_id: Uuid,
        name: &str,
        description: Option<String>,
        permissions: Option<Vec<String>>,
        clone_from: Option<&str>,
        caller: &[String],
    ) -> Outcome<Detail> {
        let permissions = match (permissions, clone_from) {
            (Some(keys), _) => {
                let declared = Catalogue::declared(db).await?;
                if let Err(err) = check_permissions(&declared, &keys, caller, &[]) {
                    return Ok(Err(err));
                }
                tidy(keys)
            }
            (None, Some(source)) => {
                let Some(source) = Self::find_visible(db, tenant_id, source).await? else {
                    return Ok(Err(RoleError::NotFound));
                };
                let keys = source.permission_keys();
                // A copy may hold keys outside the catalogue (`*`), but the
                // caller must hold everything they copy, apart from keys of an
                // extension the company has not enabled.
                let declared = Catalogue::declared(db).await?;
                let enabled = Catalogue::enabled(db, Some(tenant_id)).await?;
                if let Some(key) = keys.iter().find(|key| {
                    let disabled = declared.is_known(key) && !enabled.is_known(key);
                    !disabled && !grants(caller, key)
                }) {
                    return Ok(Err(RoleError::CannotGrant(key.clone())));
                }
                tidy(keys)
            }
            (None, None) => return Ok(Err(RoleError::PermissionsRequired)),
        };
        if Self::name_taken(db, tenant_id, name, None).await? {
            return Ok(Err(RoleError::NameTaken));
        }
        let role = ActiveModel {
            tenant_id: ActiveValue::set(Some(tenant_id)),
            name: ActiveValue::set(name.to_string()),
            role_type: ActiveValue::set(CUSTOM.to_string()),
            description: ActiveValue::set(description),
            permissions: ActiveValue::set(serde_json::json!(permissions)),
            ..Default::default()
        }
        .insert(db)
        .await?;
        Ok(Ok(Detail {
            role,
            user_count: 0,
        }))
    }

    /// Changes a custom role.
    ///
    /// # Errors
    ///
    /// `IsSystem`, `NameTaken`, `UnknownPermission` or `CannotGrant` (only
    /// for permissions the role does not hold yet).
    pub async fn update_custom(
        self,
        db: &DatabaseConnection,
        tenant_id: Uuid,
        changes: Changes,
        caller: &[String],
    ) -> Outcome<Detail> {
        if self.is_system() {
            return Ok(Err(RoleError::IsSystem));
        }
        if let Some(name) = &changes.name {
            if Self::name_taken(db, tenant_id, name, Some(self.id)).await? {
                return Ok(Err(RoleError::NameTaken));
            }
        }
        let current = self.permission_keys();
        if let Some(keys) = &changes.permissions {
            let declared = Catalogue::declared(db).await?;
            if let Err(err) = check_permissions(&declared, keys, caller, &current) {
                return Ok(Err(err));
            }
        }
        let mut role = self.into_active_model();
        if let Some(name) = changes.name {
            role.name = ActiveValue::set(name);
        }
        if let Some(description) = changes.description {
            role.description = ActiveValue::set(description);
        }
        if let Some(keys) = changes.permissions {
            role.permissions = ActiveValue::set(serde_json::json!(tidy(keys)));
        }
        let role = role.update(db).await?;
        Ok(Ok(role.detail(db, tenant_id).await?))
    }

    /// Deletes a custom role nobody holds.
    ///
    /// # Errors
    ///
    /// `IsSystem` or `InUse` (held by a member, or named by a pending
    /// invitation).
    pub async fn delete_custom(self, db: &DatabaseConnection, tenant_id: Uuid) -> Outcome<()> {
        if self.is_system() {
            return Ok(Err(RoleError::IsSystem));
        }
        let held = membership_roles::Entity::find()
            .filter(membership_roles::Column::RoleId.eq(self.id))
            .count(db)
            .await?;
        let invited = invitations::Entity::find()
            .filter(invitations::Column::TenantId.eq(tenant_id))
            .filter(invitations::Column::AcceptedAt.is_null())
            .all(db)
            .await?
            .iter()
            .any(|invitation| invitation.role_uuids().contains(&self.id));
        if held > 0 || invited {
            return Ok(Err(RoleError::InUse));
        }
        self.delete(db).await?;
        Ok(Ok(()))
    }
}
