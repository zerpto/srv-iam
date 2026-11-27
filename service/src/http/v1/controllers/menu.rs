use axum::http::StatusCode;
use loco_rs::prelude::*;
use uuid::Uuid;
use zerpto_core::{
    helpers::json,
    http::{envelope::EmptyMeta, problem::ApiResult},
};

use crate::{
    extensions::manifest::{MenuItem, SettingsMenuItem},
    http::{auth_session::AuthSession, v1::response::extensions::Menu},
    models::{menu, permissions::Catalogue, plan, tenant_extensions},
    plans::provider::provider,
};

/// `GET /v1/iam/menu`: the sidebar and the settings menu for the signed-in user
/// in their company. Before they have a company, only the core entries that need
/// no permission.
///
/// # Errors
///
/// Fails when the user or their company cannot be loaded, or the plan provider
/// cannot answer.
#[debug_handler]
pub async fn show(State(ctx): State<AppContext>, session: AuthSession) -> ApiResult {
    session.user(&ctx).await?;
    let menu = if let Some(scope) = session.scope(&ctx).await? {
        let plans = provider(&ctx)?;
        let plan = plans.plan(&ctx, scope.tenant.id).await?;
        let read_only = plan.locked && !plan::is_owner(&scope.roles);
        let features = if plan.locked {
            plans.features(&ctx).await?
        } else {
            Vec::new()
        };
        let tenant_id = Some(scope.tenant.id);
        let allowed = Catalogue::enabled(&ctx.db, tenant_id).await?.allowed(
            &scope.permissions(),
            features,
            plan.locked,
            read_only,
        );
        let (entries, settings) = declared(&ctx, tenant_id).await?;
        menu::build(entries, settings, &allowed, read_only)
    } else {
        let (entries, settings) = declared(&ctx, None).await?;
        menu::build(entries, settings, &[], false)
    };
    Ok(json(StatusCode::OK, Menu::from(menu), EmptyMeta::default()))
}

/// The sidebar and settings entries declared by the extensions active for the
/// tenant; without a tenant, by the core extensions.
async fn declared(
    ctx: &AppContext,
    tenant_id: Option<Uuid>,
) -> ModelResult<(Vec<MenuItem>, Vec<SettingsMenuItem>)> {
    let mut entries = Vec::new();
    let mut settings = Vec::new();
    for extension in tenant_extensions::Model::active(&ctx.db, tenant_id).await? {
        if let Some(frontend) = extension.live_manifest()?.and_then(|m| m.frontend) {
            entries.extend(frontend.menu);
            settings.extend(frontend.settings_menu);
        }
    }
    Ok((entries, settings))
}
