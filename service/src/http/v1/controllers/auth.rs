use axum::http::StatusCode;
use loco_rs::prelude::*;
use serde_json::json;
use zerpto_core::{
    helpers::json,
    http::{
        envelope::EmptyMeta,
        problem::{ApiResult, Problem},
    },
    relay::message::RelayMessage,
    validation::ValidatedJson,
};

use crate::{
    http::{
        auth_session::AuthSession,
        v1::{
            requests::{
                auth::{RefreshRequest, SignInRequest, SignUpRequest},
                validation::rules::normalize_email,
            },
            response::{
                auth::{AuthAccount, LoginResult, Me, RegistrationMeta, TokenPair},
                tenants::TenantSummary,
            },
        },
    },
    models::{
        memberships,
        permissions::Catalogue,
        plan,
        scope::Scope,
        sessions::{self, RefreshError},
        users::{self, NewUser},
    },
    plans::provider::provider,
    settings::Settings,
};

fn invalid_credentials() -> Problem {
    Problem::unauthorized("invalid_credentials", "Email or password is incorrect.")
}

/// `POST /v1/auth/sign-up`: create a user in `pending_verification` and email
/// the confirmation link. No tenant is created and no tokens are returned.
///
/// # Errors
///
/// `409` when the email is already registered. Fails too when the user cannot
/// be saved or the confirmation message cannot be queued.
#[debug_handler]
pub async fn sign_up(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<SignUpRequest>,
) -> ApiResult {
    let email = normalize_email(&req.email);
    let user = match users::Model::create_with_password(
        &ctx.db,
        &NewUser {
            email: &email,
            name: req.name.trim(),
            password: &req.password,
            email_verified: false,
        },
    )
    .await
    {
        Ok(user) => user,
        Err(ModelError::EntityAlreadyExists) => {
            return Err(Problem::conflict(
                "email_taken",
                "This email address is already registered.",
            )
            .field("email"));
        }
        Err(err) => return Err(err.into()),
    };

    let (user, token) = user
        .into_active_model()
        .set_email_verification_sent(&ctx.db)
        .await?;
    // After the write, so a failed sign-up never sends a message.
    RelayMessage {
        channel: "email".to_string(),
        recipient: user.email.clone(),
        template: "auth.verify_email".to_string(),
        tenant_id: None,
        data: json!({
            "name": user.name,
            "verify_url": format!("{}?token={token}", Settings::load(&ctx)?.url("verify-email")),
        }),
    }
    .send(&ctx)
    .await?;

    Ok(json(
        StatusCode::CREATED,
        AuthAccount::from(&user),
        RegistrationMeta::login(),
    ))
}

/// `POST /v1/auth/sign-in`: exchange email and password for tokens.
///
/// # Errors
///
/// `401` for an unknown email, a wrong password or an unconfirmed email, and
/// `403` when the user is deactivated in every company. Fails too when the
/// session cannot be started.
#[debug_handler]
pub async fn sign_in(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<SignInRequest>,
) -> ApiResult {
    // Unknown email, wrong password and unconfirmed email all answer the same,
    // so the endpoint cannot be used to find out who has an account.
    let user = users::Model::find_by_email(&ctx.db, &normalize_email(&req.email))
        .await?
        .ok_or_else(invalid_credentials)?;
    if !user.verify_password(&req.password) || !user.is_verified() {
        return Err(invalid_credentials());
    }
    signed_in(&ctx, &user).await
}

/// Starts a session for a user who proved who they are. A user who has not
/// onboarded yet has no tenant: they still sign in, with `tenant: null`, so
/// they can create one.
///
/// Public so a service that extends this one can add a login method (such as
/// single sign-on) that starts sessions exactly as these do.
///
/// # Errors
///
/// `403 account_deactivated` when every membership is deactivated.
pub async fn signed_in(ctx: &AppContext, user: &users::Model) -> ApiResult {
    let scope =
        if let Some(membership) = memberships::Model::find_for_sign_in(&ctx.db, user.id).await? {
            let membership = membership.into_active_model().touch(&ctx.db).await?;
            Some(Scope::load(&ctx.db, membership).await?)
        } else {
            // Deactivated everywhere is different from never having joined.
            if !memberships::Model::list_for_user(&ctx.db, user.id)
                .await?
                .is_empty()
            {
                return Err(Problem::forbidden(
                    "account_deactivated",
                    "Your access to this company has been deactivated.",
                ));
            }
            None
        };
    let tokens = sessions::start(ctx, user, scope.as_ref()).await?;

    Ok(json(
        StatusCode::OK,
        LoginResult::new(tokens, user, scope.as_ref()),
        EmptyMeta::default(),
    ))
}

/// `POST /v1/auth/refresh`: trade a refresh token for a new token pair.
///
/// # Errors
///
/// `401` when the refresh token is not valid. Fails too when the session cannot
/// be rotated.
#[debug_handler]
pub async fn refresh(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<RefreshRequest>,
) -> ApiResult {
    match sessions::rotate(&ctx, &req.refresh_token).await? {
        Ok(tokens) => Ok(json(
            StatusCode::OK,
            TokenPair::from(tokens),
            EmptyMeta::default(),
        )),
        Err(RefreshError::Invalid) => Err(Problem::unauthorized(
            "invalid_token",
            "This refresh token is not valid.",
        )),
    }
}

/// `DELETE /v1/auth/sign-out`: end the session.
///
/// # Errors
///
/// Fails when the session cannot be revoked.
#[debug_handler]
pub async fn sign_out(State(ctx): State<AppContext>, session: AuthSession) -> ApiResult {
    sessions::revoke(&ctx, &session.sid).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `GET /v1/auth/me`: the signed-in user, tenant and effective permissions.
///
/// # Errors
///
/// Fails when the user or their company cannot be loaded, or the plan provider
/// cannot answer.
#[debug_handler]
pub async fn me(State(ctx): State<AppContext>, session: AuthSession) -> ApiResult {
    let user = session.user(&ctx).await?;
    let scope = session.scope(&ctx).await?;

    // The plan provider owns the plan; IAM knows the owner, so it combines the
    // two. A provider that cannot answer fails the request.
    let plans = provider(&ctx)?;
    let trial_available = plans.trial_available(&ctx, user.id).await?;

    let (access, permissions) = match &scope {
        Some(scope) => {
            let plan = plans.plan(&ctx, scope.tenant.id).await?;
            let owner = plan::is_owner(&scope.roles);
            let read_only = plan.locked && !owner;
            let features = if plan.locked {
                plans.features(&ctx).await?
            } else {
                Vec::new()
            };
            let permissions = Catalogue::enabled(&ctx.db, Some(scope.tenant.id))
                .await?
                .allowed(&scope.permissions(), features, plan.locked, read_only);
            (plan::access(plan.locked, owner), permissions)
        }
        None => ("full", Vec::new()),
    };
    Ok(json(
        StatusCode::OK,
        Me {
            user: (&user).into(),
            tenant: scope.as_ref().map(TenantSummary::from),
            permissions,
            access: access.to_string(),
            trial_available,
            phone_verified: user.is_phone_verified(),
        },
        EmptyMeta::default(),
    ))
}
