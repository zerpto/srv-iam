use axum::http::StatusCode;
use loco_rs::prelude::*;
use zerpto_core::{
    helpers::json,
    http::{envelope::EmptyMeta, problem::ApiResult},
};

use crate::http::v1::response::health::Health;

/// `GET /v1/health`: the service is up. No credential, no tenant data.
///
/// # Errors
///
/// Never fails; the `Result` is the shape every handler returns.
#[debug_handler]
pub async fn health() -> ApiResult {
    Ok(json(StatusCode::OK, Health::ok(), EmptyMeta::default()))
}
