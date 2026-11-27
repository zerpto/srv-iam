use loco_rs::prelude::*;

use super::{memberships, roles, tenants, user_permission_grants};

/// What a session is scoped to, resolved from the database: the tenant, the
/// roles the member holds in it and their direct permission grants.
#[derive(Debug)]
pub struct Scope {
    pub membership: memberships::Model,
    pub tenant: tenants::Model,
    pub roles: Vec<roles::Model>,
    /// Permission keys granted to the member directly and not expired.
    pub grants: Vec<String>,
}

impl Scope {
    /// Resolves a membership into its tenant, roles and grants.
    ///
    /// # Errors
    ///
    /// When a query fails.
    pub async fn load<C: ConnectionTrait>(
        db: &C,
        membership: memberships::Model,
    ) -> ModelResult<Self> {
        Ok(Self {
            tenant: membership.tenant(db).await?,
            roles: membership.roles(db).await?,
            grants: user_permission_grants::Model::current_keys(db, membership.id).await?,
            membership,
        })
    }

    /// What the member may do: the permissions of all their roles plus their
    /// direct grants, sorted and without duplicates.
    #[must_use]
    pub fn permissions(&self) -> Vec<String> {
        let mut keys = roles::effective_permissions(&self.roles);
        keys.extend(self.grants.iter().cloned());
        keys.sort();
        keys.dedup();
        keys
    }
}
