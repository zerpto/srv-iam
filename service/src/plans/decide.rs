//! The one answer to "may this member do this?", shared by the HTTP guard
//! (`http::auth_session`) and the gRPC `Authorization` service, so the two cannot
//! differ.
//!
//! The pure plan rules it applies are in `models::plan`.

use loco_rs::app::AppContext;
use zerpto_core::http::problem::Problem;

use super::provider::provider;
use crate::models::{
    permissions::Catalogue,
    plan::{self, Feature, Plan},
    roles::ALL_PERMISSIONS,
    scope::Scope,
};

/// What an active member may do with a call that needs one of some keys.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    Allowed,
    /// The member holds none of the keys.
    Denied,
    /// Only a paid feature allows the call, and the plan is locked.
    PlanRequired(Feature),
    /// A write by a non-owner while the plan is locked.
    ReadOnly,
}

/// Decides whether the member of `scope` holds ANY of `keys` (or `*` for
/// everything) and the tenant's `plan` allows the call.
///
/// When the plan is locked (the tenant is free or overdue, as the plan
/// provider says), a call that only paid features allow is refused, and so is
/// a write by anyone but an owner.
///
/// # Errors
///
/// When the feature catalogue cannot be had.
pub async fn decide(
    ctx: &AppContext,
    scope: &Scope,
    plan: &Plan,
    keys: &[&str],
) -> Result<Verdict, Problem> {
    let held = scope.permissions();
    if !keys.iter().any(|key| {
        held.iter()
            .any(|granted| granted == ALL_PERMISSIONS || granted == key)
    }) {
        return Ok(Verdict::Denied);
    }
    if !plan.locked {
        return Ok(Verdict::Allowed);
    }
    let overrides = provider(ctx)?.features(ctx).await?;
    let catalogue = Catalogue::enabled(&ctx.db, Some(scope.tenant.id))
        .await?
        .features(overrides);
    if let Some(feature) = plan::paid_feature_for(&catalogue, keys) {
        return Ok(Verdict::PlanRequired(feature.clone()));
    }
    let writes = keys.iter().all(|key| plan::is_write(&catalogue, key));
    if writes && !plan::is_owner(&scope.roles) {
        return Ok(Verdict::ReadOnly);
    }
    Ok(Verdict::Allowed)
}
