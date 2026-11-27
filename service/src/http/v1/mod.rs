//! The `/v1` API. To add a version, copy this layout to `v2/`, add
//! `pub mod v2;` in `http/mod.rs` and mount its routes in `App::routes`.

pub mod controllers;
pub mod requests;
pub mod response;
pub mod routing;
