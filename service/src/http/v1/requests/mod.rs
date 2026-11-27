//! `/v1` request bodies. Rules go on the fields; `ValidatedJson` checks them
//! before the handler runs.

pub mod address;
pub mod auth;
pub mod extensions;
pub mod invitation;
pub mod reset_password;
pub mod roles;
pub mod tenants;
pub mod user_permissions;
pub mod users;
pub mod validation;
pub mod verify;
