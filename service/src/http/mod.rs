//! This service's HTTP API, one directory per version.
//!
//! The error type, response envelope, paging and the stateless session extractor
//! come from `zerpto_core::http`; `auth_session` is IAM's own, because only IAM
//! knows about sessions (`sid`).

pub mod auth_session;
pub mod ids;
pub mod platform_key;
pub mod v1;
