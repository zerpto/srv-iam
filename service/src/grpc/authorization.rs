//! Whether a user may do something in a tenant (`zerpto.iam.v1.Authorization`),
//! asked by every other service on each request.

use loco_rs::app::AppContext;
use tonic::{Request, Response, Status};
use uuid::Uuid;
use zerpto_core::http::ids::ApiIds;
use zerpto_proto::iam::v1::{
    authorization_server::Authorization, AuthorizeRequest, AuthorizeResponse, Decision,
};

use crate::{
    http::ids::{TENANT, USER},
    models::{memberships, scope::Scope},
    plans::{
        decide::{decide, Verdict},
        provider::provider,
    },
};

/// Answers `Authorization` from this service's database and the registered plan
/// provider. Nothing is cached.
pub struct AuthorizationService {
    pub ctx: AppContext,
}

/// The ids of this call are API ids, unlike the other calls of this service:
/// the contract says so, and callers forward what the access token carries.
fn parse_api_id(field: &str, prefix: &str, value: &str) -> Result<Uuid, Status> {
    ApiIds::new(prefix)
        .parse(value)
        .ok_or_else(|| Status::invalid_argument(format!("{field} must be a `{prefix}-<uuid>` id")))
}

fn failed(error: impl std::fmt::Debug) -> Status {
    tracing::error!(?error, "authorization check failed");
    Status::internal("could not check the permission")
}

fn answer(decision: Decision) -> AuthorizeResponse {
    AuthorizeResponse {
        decision: decision.into(),
        feature_key: String::new(),
        feature_name: String::new(),
    }
}

impl AuthorizationService {
    async fn check(
        &self,
        user_id: Uuid,
        tenant_id: Uuid,
        keys: &[&str],
    ) -> Result<AuthorizeResponse, Status> {
        let ctx = &self.ctx;
        let Some(membership) = memberships::Model::find_in_tenant(&ctx.db, user_id, tenant_id)
            .await
            .map_err(failed)?
        else {
            return Ok(answer(Decision::Denied));
        };
        if !membership.is_active() {
            return Ok(answer(Decision::AccountDeactivated));
        }
        let scope = Scope::load(&ctx.db, membership).await.map_err(failed)?;
        let plan = provider(ctx)
            .map_err(failed)?
            .plan(ctx, tenant_id)
            .await
            .map_err(failed)?;
        Ok(
            match decide(ctx, &scope, &plan, keys).await.map_err(failed)? {
                Verdict::Allowed => answer(Decision::Allowed),
                // The contract has no decision for a read-only member, so they
                // are denied.
                Verdict::Denied | Verdict::ReadOnly => answer(Decision::Denied),
                Verdict::PlanRequired(feature) => AuthorizeResponse {
                    decision: Decision::PlanRequired.into(),
                    feature_key: feature.key,
                    feature_name: feature.name,
                },
            },
        )
    }
}

#[tonic::async_trait]
impl Authorization for AuthorizationService {
    /// Denied for someone who is not a member of the tenant.
    async fn authorize(
        &self,
        request: Request<AuthorizeRequest>,
    ) -> Result<Response<AuthorizeResponse>, Status> {
        let request = request.into_inner();
        let user_id = parse_api_id("user_id", USER, &request.user_id)?;
        let tenant_id = parse_api_id("tenant_id", TENANT, &request.tenant_id)?;
        let keys: Vec<&str> = request.permission_keys.iter().map(String::as_str).collect();
        self.check(user_id, tenant_id, &keys)
            .await
            .map(Response::new)
    }
}
