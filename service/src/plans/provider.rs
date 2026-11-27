use std::sync::Arc;

use async_trait::async_trait;
use loco_rs::{app::AppContext, Error};
use uuid::Uuid;
use zerpto_core::http::problem::Problem;

use crate::models::plan::{Feature, Plan};

/// Answers what a tenant's plan allows.
///
/// One provider is kept in `ctx.shared_store` as an `Arc<dyn PlanProvider>`.
/// Never call it inside a database transaction: it may be a network call. An
/// implementation that cannot answer must fail closed with a `Problem`.
#[async_trait]
pub trait PlanProvider: Send + Sync {
    /// The tenant's plan state.
    ///
    /// # Errors
    ///
    /// A `Problem` the caller returns as is, when the plan cannot be had.
    async fn plan(&self, ctx: &AppContext, tenant_id: Uuid) -> Result<Plan, Problem>;

    /// The feature catalogue: permission keys with their plan and kind.
    ///
    /// # Errors
    ///
    /// As [`plan`](Self::plan).
    async fn features(&self, ctx: &AppContext) -> Result<Vec<Feature>, Problem>;

    /// Whether `user_id` may still start their one trial. Works before the
    /// user has a tenant.
    ///
    /// # Errors
    ///
    /// As [`plan`](Self::plan).
    async fn trial_available(&self, ctx: &AppContext, user_id: Uuid) -> Result<bool, Problem>;
}

/// The registered [`PlanProvider`].
///
/// # Errors
///
/// `500` when none is registered. There is no silent default: a missing
/// provider must not unlock anything.
pub fn provider(ctx: &AppContext) -> Result<Arc<dyn PlanProvider>, Problem> {
    ctx.shared_store
        .get::<Arc<dyn PlanProvider>>()
        .ok_or_else(|| Problem::from(Error::string("no plan provider is registered")))
}
