use axum::http::StatusCode;
use loco_rs::prelude::*;
use zerpto_core::{
    helpers::json,
    http::{
        envelope::EmptyMeta,
        problem::{ApiResult, Problem},
    },
    validation::ValidatedJson,
};

use crate::{
    http::{
        auth_session::{Access, AuthSession},
        v1::{
            requests::user_permissions::SetUserPermissionsRequest,
            response::user_permissions::{UserPermissionGrant, UserPermissions},
        },
    },
    models::{
        memberships,
        permissions::Catalogue,
        plan,
        user_permission_grants::{self, GrantError},
    },
    plans::provider::provider,
};

const READ: [&str; 2] = ["iam.users.read", "iam.users.manage"];
const MANAGE: [&str; 1] = ["iam.users.manage"];

fn grant_problem(err: &GrantError) -> Problem {
    match err {
        GrantError::UnknownPermission(key) => Problem::unprocessable(
            "unknown_permission",
            format!("`{key}` is not a permission. See GET /v1/iam/permissions."),
        )
        .field("grants"),
        GrantError::CannotGrant(key) => Problem::forbidden(
            "cannot_grant",
            format!("You cannot give `{key}` because you do not hold it yourself."),
        ),
        GrantError::ExpiresInThePast(key) => Problem::unprocessable(
            "invalid_expires_at",
            format!("The grant of `{key}` must expire in the future."),
        )
        .field("grants"),
    }
}

/// A member (not an invitee, who has no permissions yet) of the tenant.
async fn find_member(
    ctx: &AppContext,
    access: &Access,
    id: &str,
) -> Result<memberships::Model, Problem> {
    memberships::Model::find_by_api_id(&ctx.db, access.tenant_id(), id)
        .await?
        .ok_or_else(|| Problem::not_found("No such user."))
}

/// The member's grants and final permissions: only those of core and enabled
/// extensions. On a free or overdue plan the final set also drops paid
/// permissions, and every write for a non-owner.
async fn render(ctx: &AppContext, access: &Access, membership: &memberships::Model) -> ApiResult {
    let direct = user_permission_grants::Model::current_for(&ctx.db, membership.id)
        .await?
        .into_iter()
        .map(UserPermissionGrant::from)
        .collect();
    let held = membership.effective_permissions(&ctx.db).await?;
    let locked = access.plan.locked;
    let (features, read_only) = if locked {
        let owner = plan::is_owner(&membership.roles(&ctx.db).await?);
        (provider(ctx)?.features(ctx).await?, !owner)
    } else {
        (Vec::new(), false)
    };
    let effective = Catalogue::enabled(&ctx.db, Some(access.tenant_id()))
        .await?
        .allowed(&held, features, locked, read_only);
    Ok(json(
        StatusCode::OK,
        UserPermissions { direct, effective },
        EmptyMeta::default(),
    ))
}

/// `GET /v1/iam/users/{id}/permissions`: direct grants and the final set.
///
/// # Errors
///
/// `403` without the permission to read users, and `404` when the company has
/// no such member.
#[debug_handler]
pub async fn show(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult {
    let access = session.authorize(&ctx, &READ).await?;
    let membership = find_member(&ctx, &access, &id).await?;
    render(&ctx, &access, &membership).await
}

/// `PUT /v1/iam/users/{id}/permissions`: replace the direct grants.
///
/// # Errors
///
/// `403` without the permission to manage users, `404` for an unknown member,
/// and a refusal for grants that are invalid or not allowed.
#[debug_handler]
pub async fn replace(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<SetUserPermissionsRequest>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let grants = req.into_domain()?;
    let membership = find_member(&ctx, &access, &id).await?;
    user_permission_grants::Model::replace_for(
        &ctx.db,
        &membership,
        &grants,
        &access.user,
        &access.permissions(),
    )
    .await?
    .map_err(|err| grant_problem(&err))?;
    render(&ctx, &access, &membership).await
}
