use axum::http::StatusCode;
use loco_rs::prelude::*;
use zerpto_core::{
    helpers::json,
    http::{envelope::EmptyMeta, problem::ApiResult},
};

use crate::{
    http::{auth_session::AuthSession, v1::response::roles::Permission},
    models::permissions::Catalogue,
    plans::provider::provider,
};

/// `GET /v1/iam/permissions`: the permissions the caller's company can use.
///
/// These are the permissions of the core extensions and of those the company
/// enabled (only the core ones before the caller has a company), with the plan each
/// needs. Any signed-in user may read it; `locked` is true when the company is on a
/// plan that lacks the permission.
///
/// # Errors
///
/// Fails when the user or the catalogue cannot be loaded, or the plan provider
/// cannot answer.
#[debug_handler]
pub async fn list(State(ctx): State<AppContext>, session: AuthSession) -> ApiResult {
    session.user(&ctx).await?;
    let plans = provider(&ctx)?;
    let locked = match session.tenant_id() {
        Some(tenant_id) => plans.plan(&ctx, tenant_id).await?.locked,
        None => false,
    };
    let catalogue = Catalogue::enabled(&ctx.db, session.tenant_id()).await?;
    let features = catalogue.features(plans.features(&ctx).await?);
    let permissions: Vec<Permission> = catalogue
        .permissions()
        .iter()
        .map(|permission| Permission::new(permission, &features, locked))
        .collect();
    Ok(json(StatusCode::OK, permissions, EmptyMeta::default()))
}
