use std::future::{self, Future};

use axum::{extract::FromRequestParts, http::request::Parts};
use loco_rs::{app::AppContext, auth::jwt};
use uuid::Uuid;
use zerpto_core::http::{ids::ApiIds, problem::Problem};

use crate::http::ids::{TENANT, USER};
use crate::models::{memberships, plan::Plan, scope::Scope, users};
use crate::plans::{
    decide::{decide, Verdict},
    provider::provider,
};

/// The signed-in session, read from the `Authorization: Bearer` access token.
///
/// Like `zerpto_core::http::session::CurrentSession` it checks only the
/// signature and expiry, but it also reads `sid`, the session id IAM puts in
/// the token and that no other service needs. Use [`user`](Self::user) and
/// [`scope`](Self::scope) to load what a handler needs.
#[derive(Debug)]
pub struct AuthSession {
    pub sid: String,
    user_id: Uuid,
    /// `None` for a session from before onboarding (no tenant yet).
    tenant_id: Option<Uuid>,
}

fn unauthorized() -> Problem {
    Problem::unauthorized("invalid_token", "Missing, invalid or expired access token.")
}

impl FromRequestParts<AppContext> for AuthSession {
    type Rejection = Problem;

    fn from_request_parts(
        parts: &mut Parts,
        ctx: &AppContext,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        future::ready(Self::from_parts(parts, ctx))
    }
}

impl AuthSession {
    /// Reads and checks the access token of the request.
    fn from_parts(parts: &Parts, ctx: &AppContext) -> Result<Self, Problem> {
        let token = parts
            .headers
            .get(axum::http::header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .ok_or_else(unauthorized)?;

        let secret = &ctx.config.get_jwt_config()?.secret;
        let data = jwt::JWT::new(secret)
            .validate(token)
            .map_err(|_| unauthorized())?;

        let claim = |name: &str| {
            data.claims
                .claims
                .get(name)
                .and_then(|value| value.as_str())
                .map(str::to_string)
        };
        Ok(Self {
            sid: claim("sid").ok_or_else(unauthorized)?,
            user_id: ApiIds::new(USER)
                .parse(&data.claims.pid)
                .ok_or_else(unauthorized)?,
            tenant_id: claim("tenant_id").and_then(|id| ApiIds::new(TENANT).parse(&id)),
        })
    }

    /// The tenant the token is scoped to; `None` before onboarding.
    #[must_use]
    pub const fn tenant_id(&self) -> Option<Uuid> {
        self.tenant_id
    }

    /// # Errors
    ///
    /// `401` when the user no longer exists.
    pub async fn user(&self, ctx: &AppContext) -> Result<users::Model, Problem> {
        users::Model::find_by_id(&ctx.db, self.user_id)
            .await?
            .ok_or_else(unauthorized)
    }

    /// The tenant, roles and grants the session is scoped to, read fresh from
    /// the database so role changes apply at once. `None` when the session has
    /// no tenant yet.
    ///
    /// # Errors
    ///
    /// `401` when the user left the tenant; `403` when deactivated in it.
    pub async fn scope(&self, ctx: &AppContext) -> Result<Option<Scope>, Problem> {
        let Some(tenant_id) = self.tenant_id else {
            return Ok(None);
        };
        let membership = memberships::Model::find_in_tenant(&ctx.db, self.user_id, tenant_id)
            .await?
            .ok_or_else(unauthorized)?;
        if !membership.is_active() {
            return Err(Problem::forbidden(
                "account_deactivated",
                "Your access to this company has been deactivated.",
            ));
        }
        Ok(Some(Scope::load(&ctx.db, membership).await?))
    }
}

/// A signed-in user who works in a tenant, with what they may do there.
#[derive(Debug)]
pub struct Access {
    pub user: users::Model,
    pub scope: Scope,
    /// The tenant's plan state, asked from the plan provider once per request.
    pub plan: Plan,
}

impl Access {
    /// The tenant's id.
    #[must_use]
    pub const fn tenant_id(&self) -> Uuid {
        self.scope.tenant.id
    }

    /// What the caller may do in the tenant: their roles' permissions plus
    /// their direct grants.
    #[must_use]
    pub fn permissions(&self) -> Vec<String> {
        self.scope.permissions()
    }
}

impl AuthSession {
    /// Requires a tenant, with no check on what the caller may do there.
    ///
    /// # Errors
    ///
    /// `401` for an unknown user, `403 tenant_required` without a tenant,
    /// `403 account_deactivated`.
    pub async fn access(&self, ctx: &AppContext) -> Result<Access, Problem> {
        let user = self.user(ctx).await?;
        let scope = self.scope(ctx).await?.ok_or_else(|| {
            Problem::forbidden(
                "tenant_required",
                "This action needs a company; finish onboarding first.",
            )
        })?;
        let plan = provider(ctx)?.plan(ctx, scope.tenant.id).await?;
        Ok(Access { user, scope, plan })
    }

    /// Requires a tenant and that the caller holds ANY of `keys` (or `*`
    /// for everything) there. Read fresh from the database, so a role change
    /// applies at once.
    ///
    /// When the plan is locked (the tenant is free or overdue, as the plan
    /// provider says), a call that only paid features allow is refused, and so
    /// is a write by anyone but an owner.
    ///
    /// # Errors
    ///
    /// `401` for an unknown user, `403 tenant_required` without a tenant,
    /// `403 account_deactivated`, `403 permission_denied`, `403 plan_required`,
    /// `403 read_only_user`, and whatever the plan provider refuses with.
    pub async fn authorize(&self, ctx: &AppContext, keys: &[&str]) -> Result<Access, Problem> {
        let access = self.access(ctx).await?;
        match decide(ctx, &access.scope, &access.plan, keys).await? {
            Verdict::Allowed => Ok(access),
            Verdict::Denied => Err(Problem::forbidden(
                "permission_denied",
                "You do not have permission to do this.",
            )),
            Verdict::PlanRequired(feature) => Err(Problem::forbidden(
                "plan_required",
                format!(
                    "{} (`{}`) is a paid feature. Start the free trial or subscribe under Billing to unlock it.",
                    feature.name, feature.key
                ),
            )),
            Verdict::ReadOnly => Err(Problem::forbidden(
                "read_only_user",
                "This company is on the free plan or overdue, so only an owner can change data. Ask an owner to subscribe or pay the open invoice under Billing.",
            )),
        }
    }
}
