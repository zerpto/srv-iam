//! The API id prefixes of the records IAM owns.
//!
//! `zerpto-core` registers none: a prefix belongs to the owning service, and the
//! list of all prefixes (checked for collisions across services) is kept in the
//! `com.zerpto.openapi` repo. Encode and parse with
//! `zerpto_core::http::ids::ApiIds::new(PREFIX)`.

/// A user. Shared with every service that shows a user id.
pub const USER: &str = "usr";
/// A tenant (company). Shared with every service.
pub const TENANT: &str = "ten";
/// A role.
pub const ROLE: &str = "rol";
/// A user's membership in a tenant.
pub const MEMBERSHIP: &str = "mem";
/// A registered extension.
pub const EXTENSION: &str = "ext";
/// A region, owned by the reference-data service; IAM only stores and echoes
/// the id of a tenant address's region.
pub const REGION: &str = "reg";
