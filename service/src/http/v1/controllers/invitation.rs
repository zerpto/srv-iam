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
        auth_session::AuthSession,
        v1::{
            requests::invitation::{AcceptInviteExistingRequest, AcceptInviteRequest},
            response::{auth::LoginResult, users::User},
        },
    },
    models::{
        invitations::{self, InvitationError},
        members, memberships, roles,
        scope::Scope,
        sessions,
    },
    plans::provider::provider,
};

/// `POST /v1/auth/accept-invite`: create the invited account and sign it in.
///
/// # Errors
///
/// `400` for an invalid invitation, `422` for an expired one and `409` when the
/// email already has an account. Fails too when the account or session cannot
/// be created.
#[debug_handler]
pub async fn accept_invite(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<AcceptInviteRequest>,
) -> ApiResult {
    let invitation = invitations::Model::find_by_token(&ctx.db, &req.token)
        .await?
        .map_err(|err| invitation_problem(&err))?;
    let tenant_id = invitation.tenant_id;

    let user = match invitation
        .accept(&ctx.db, req.name.trim(), &req.password)
        .await
    {
        Ok(user) => user,
        Err(ModelError::EntityAlreadyExists) => {
            return Err(Problem::conflict(
                "account_exists",
                "This email address already has an account. Sign in and join from there.",
            ));
        }
        Err(err) => return Err(err.into()),
    };
    let membership = memberships::Model::find_in_tenant(&ctx.db, user.id, tenant_id)
        .await?
        .ok_or_else(|| Problem::from(Error::string("membership missing after accept")))?;
    let scope = Scope::load(&ctx.db, membership).await?;
    let tokens = sessions::start(&ctx, &user, Some(&scope)).await?;

    Ok(json(
        StatusCode::CREATED,
        LoginResult::new(tokens, &user, Some(&scope)),
        EmptyMeta::default(),
    ))
}

fn invitation_problem(err: &InvitationError) -> Problem {
    match err {
        InvitationError::Invalid => {
            Problem::bad_request("invalid_token", "This invitation is not valid.")
        }
        InvitationError::Expired => {
            Problem::unprocessable("token_expired", "This invitation has expired.")
        }
    }
}

/// `POST /v1/auth/accept-invite/existing`: the signed-in user joins the
/// inviting tenant. The current session does not change.
///
/// # Errors
///
/// `400` for an invalid invitation, `422` for an expired one and `403` when it
/// was sent to another email address. Fails too when the membership cannot be
/// saved.
#[debug_handler]
pub async fn accept_invite_existing(
    State(ctx): State<AppContext>,
    session: AuthSession,
    ValidatedJson(req): ValidatedJson<AcceptInviteExistingRequest>,
) -> ApiResult {
    let user = session.user(&ctx).await?;
    let invitation = invitations::Model::find_by_token(&ctx.db, &req.token)
        .await?
        .map_err(|err| invitation_problem(&err))?;
    if invitation.email != user.email {
        return Err(Problem::forbidden(
            "email_mismatch",
            "This invitation was sent to a different email address.",
        ));
    }

    let membership = invitation.accept_existing(&ctx.db, &user).await?;
    let member = members::find(&ctx.db, membership.tenant_id, &membership.api_id())
        .await?
        .ok_or_else(|| Problem::from(Error::string("membership missing after accept")))?;
    let plan = provider(&ctx)?.plan(&ctx, membership.tenant_id).await?;
    let owner_role_id = roles::Model::owner_api_id(&ctx.db).await?;
    Ok(json(
        StatusCode::OK,
        User::new(member, plan.locked, &owner_role_id),
        EmptyMeta::default(),
    ))
}
