//! This service's models.
//!
//! Every table has a single `id` primary key, a `UUIDv7` generated in
//! `ActiveModelBehavior::before_save`; the pattern is in
//! `zerpto_core::events::outbox` (`entity.rs` and `model.rs`).
//!
//! `_entities` holds the plain `SeaORM` entities; each sibling file adds the
//! domain logic of one table.

// Generated code, so the lint is allowed here, where regenerating keeps it: SeaORM needs an empty
// `Relation` enum for a table without relations.
#[allow(clippy::empty_enums)]
pub mod _entities;
pub mod address;
pub mod digest;
pub mod extensions;
pub mod invitations;
pub mod members;
pub mod membership_roles;
pub mod memberships;
pub mod menu;
pub mod outbox_events;
pub mod permissions;
pub mod phone_verifications;
pub mod plan;
pub mod roles;
pub mod scope;
pub mod sessions;
pub mod tenant_extensions;
pub mod tenants;
pub mod user_permission_grants;
pub mod users;
