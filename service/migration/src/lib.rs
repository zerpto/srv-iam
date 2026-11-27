#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;

mod m20261122_000001_users;
mod m20261122_000002_tenants;
mod m20261122_000003_roles;
mod m20261122_000004_memberships;
mod m20261122_000005_membership_roles;
mod m20261122_000006_user_permission_grants;
mod m20261122_000007_invitations;
mod m20261122_000008_phone_verifications;

mod m20261122_002359_add_profile_to_tenants;
mod m20261122_002459_seed_owner_role;
mod m20261126_000001_extensions;
mod m20261126_000002_tenant_extensions;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            // Core migrations first (the transactional outbox).
            Box::new(zerpto_core::events::outbox::Migration),
            Box::new(m20261122_000001_users::Migration),
            Box::new(m20261122_000002_tenants::Migration),
            Box::new(m20261122_000003_roles::Migration),
            Box::new(m20261122_000004_memberships::Migration),
            Box::new(m20261122_000005_membership_roles::Migration),
            Box::new(m20261122_000006_user_permission_grants::Migration),
            Box::new(m20261122_000007_invitations::Migration),
            Box::new(m20261122_000008_phone_verifications::Migration),
            Box::new(m20261122_002359_add_profile_to_tenants::Migration),
            Box::new(m20261122_002459_seed_owner_role::Migration),
            Box::new(m20261126_000001_extensions::Migration),
            Box::new(m20261126_000002_tenant_extensions::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
