use async_trait::async_trait;
use loco_rs::app::AppContext;
use uuid::Uuid;
use zerpto_core::http::problem::Problem;

use super::provider::PlanProvider;
use crate::models::plan::{Feature, Plan};

/// The community edition's plan: nothing is locked, seats are unlimited, there
/// is no feature catalogue and no trial.
#[derive(Debug, Clone, Copy, Default)]
pub struct Unlimited;

#[async_trait]
impl PlanProvider for Unlimited {
    async fn plan(&self, _ctx: &AppContext, _tenant_id: Uuid) -> Result<Plan, Problem> {
        Ok(Plan {
            locked: false,
            seat_limit: i64::MAX,
        })
    }

    async fn features(&self, _ctx: &AppContext) -> Result<Vec<Feature>, Problem> {
        Ok(Vec::new())
    }

    async fn trial_available(&self, _ctx: &AppContext, _user_id: Uuid) -> Result<bool, Problem> {
        Ok(false)
    }
}
