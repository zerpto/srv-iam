pub use super::_entities::memberships::{ActiveModel, Column, Entity, Model};
use chrono::Utc;
use loco_rs::prelude::*;
use uuid::Uuid;
use zerpto_core::http::ids::ApiIds;

use super::{_entities::membership_roles, roles, tenants, user_permission_grants};
use crate::http::ids::MEMBERSHIP;

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

impl Model {
    /// The API id: `mem-<uuid>`.
    #[must_use]
    pub fn api_id(&self) -> String {
        ApiIds::new(MEMBERSHIP).encode(&self.id)
    }

    /// Whether the person may still act in the tenant.
    #[must_use]
    pub const fn is_active(&self) -> bool {
        self.deactivated_at.is_none()
    }

    /// Every tenant the user belongs to, oldest membership first. `id` is a
    /// `UUIDv7`, so it sorts by creation time.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn list_for_user<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
    ) -> ModelResult<Vec<Self>> {
        Ok(Entity::find()
            .filter(Column::UserId.eq(user_id))
            .order_by_asc(Column::Id)
            .all(db)
            .await?)
    }

    /// The membership a user signs in to: the tenant used last, otherwise the
    /// first one they joined. `None` when the user has no active membership.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_for_sign_in<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
    ) -> ModelResult<Option<Self>> {
        Ok(Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::DeactivatedAt.is_null())
            .order_by_desc(Column::LastUsedAt)
            .order_by_asc(Column::Id)
            .one(db)
            .await?)
    }

    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_in_tenant<C: ConnectionTrait>(
        db: &C,
        user_id: Uuid,
        tenant_id: Uuid,
    ) -> ModelResult<Option<Self>> {
        Ok(Entity::find()
            .filter(Column::UserId.eq(user_id))
            .filter(Column::TenantId.eq(tenant_id))
            .one(db)
            .await?)
    }

    /// # Errors
    ///
    /// When the query fails or the tenant is gone.
    pub async fn tenant<C: ConnectionTrait>(&self, db: &C) -> ModelResult<tenants::Model> {
        tenants::Model::find_by_id(db, self.tenant_id).await
    }

    /// The membership of a tenant by its API id (`mem-<uuid>`).
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn find_by_api_id<C: ConnectionTrait>(
        db: &C,
        tenant_id: Uuid,
        api_id: &str,
    ) -> ModelResult<Option<Self>> {
        let Some(id) = ApiIds::new(MEMBERSHIP).parse(api_id) else {
            return Ok(None);
        };
        Ok(Entity::find()
            .filter(Column::TenantId.eq(tenant_id))
            .filter(Column::Id.eq(id))
            .one(db)
            .await?)
    }

    /// What the member may do: all their roles' permissions plus their
    /// unexpired direct grants, sorted and without duplicates.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn effective_permissions<C: ConnectionTrait>(
        &self,
        db: &C,
    ) -> ModelResult<Vec<String>> {
        let mut keys = roles::effective_permissions(&self.roles(db).await?);
        keys.extend(user_permission_grants::Model::current_keys(db, self.id).await?);
        keys.sort();
        keys.dedup();
        Ok(keys)
    }

    /// The ids of the roles held through this membership.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn role_ids<C: ConnectionTrait>(&self, db: &C) -> ModelResult<Vec<Uuid>> {
        Ok(membership_roles::Entity::find()
            .filter(membership_roles::Column::MembershipId.eq(self.id))
            .all(db)
            .await?
            .into_iter()
            .map(|link| link.role_id)
            .collect())
    }

    /// The roles held through this membership.
    ///
    /// # Errors
    ///
    /// When the query fails.
    pub async fn roles<C: ConnectionTrait>(&self, db: &C) -> ModelResult<Vec<roles::Model>> {
        let role_ids = self.role_ids(db).await?;
        Ok(roles::Entity::find()
            .filter(roles::Column::Id.is_in(role_ids))
            .all(db)
            .await?)
    }

    /// Adds a user to a tenant with the given roles. `id` is the new
    /// membership's id; an accepted invitation passes its own, so the `mem-`
    /// id does not change when the invitation is accepted.
    ///
    /// # Errors
    ///
    /// When an insert fails.
    pub async fn create_with_roles<C: ConnectionTrait>(
        db: &C,
        id: Uuid,
        user_id: Uuid,
        tenant_id: Uuid,
        role_ids: &[Uuid],
    ) -> ModelResult<Self> {
        let membership = ActiveModel {
            id: ActiveValue::set(id),
            user_id: ActiveValue::set(user_id),
            tenant_id: ActiveValue::set(tenant_id),
            last_used_at: ActiveValue::set(Some(Utc::now().into())),
            ..Default::default()
        }
        .insert(db)
        .await?;
        membership.replace_roles(db, role_ids).await?;
        Ok(membership)
    }

    /// Makes `role_ids` the only roles of this membership.
    ///
    /// # Errors
    ///
    /// When a write fails.
    pub async fn replace_roles<C: ConnectionTrait>(
        &self,
        db: &C,
        role_ids: &[Uuid],
    ) -> ModelResult<()> {
        membership_roles::Entity::delete_many()
            .filter(membership_roles::Column::MembershipId.eq(self.id))
            .exec(db)
            .await?;
        for role_id in role_ids {
            membership_roles::ActiveModel {
                membership_id: ActiveValue::set(self.id),
                role_id: ActiveValue::set(*role_id),
                ..Default::default()
            }
            .insert(db)
            .await?;
        }
        Ok(())
    }
}

impl ActiveModel {
    /// Records that the user just signed in to this tenant.
    ///
    /// # Errors
    ///
    /// When the update fails.
    pub async fn touch<C: ConnectionTrait>(mut self, db: &C) -> ModelResult<Model> {
        self.last_used_at = ActiveValue::set(Some(Utc::now().into()));
        Ok(self.update(db).await?)
    }
}
