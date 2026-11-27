//! Extensions: the manifest each one describes itself with, how it is checked
//! and fetched, and the core ones installed with the platform. The registry
//! itself is `models::extensions`.

pub mod core_manifests;
pub mod fetch;
pub mod manifest;
pub mod manifest_check;
pub mod sync;
