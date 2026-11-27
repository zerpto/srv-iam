use axum::http::StatusCode;
use loco_rs::prelude::*;
use serde_json::json;
use zerpto_core::{
    http::problem::{ApiResult, Problem},
    relay::message::RelayMessage,
    validation::ValidatedJson,
};

use crate::{
    http::{
        auth_session::AuthSession,
        v1::requests::{
            reset_password::{ChangePasswordRequest, ForgotPasswordRequest, ResetPasswordRequest},
            validation::rules::normalize_email,
        },
    },
    models::{sessions, users},
    settings::Settings,
};

/// `PUT /v1/auth/password`: change the password; other devices are signed out.
///
/// # Errors
///
/// `401` when the current password is wrong. Fails too when the new password
/// cannot be saved or the session cannot be renewed.
#[debug_handler]
pub async fn change_password(
    State(ctx): State<AppContext>,
    session: AuthSession,
    ValidatedJson(req): ValidatedJson<ChangePasswordRequest>,
) -> ApiResult {
    let user = session.user(&ctx).await?;
    if !user.verify_password(&req.current_password) {
        return Err(Problem::unauthorized(
            "invalid_credentials",
            "The current password is incorrect.",
        )
        .field("current_password"));
    }

    user.into_active_model()
        .change_password(&ctx.db, &req.new_password)
        .await?;
    sessions::renew(&ctx, &session.sid).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `POST /v1/auth/reset-password`: email a reset link. Always `202`, so it
/// cannot be used to find out which addresses are registered.
///
/// # Errors
///
/// Fails when the reset token cannot be saved or the message cannot be queued.
/// An unknown address is not an error.
#[debug_handler]
pub async fn forgot_password(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<ForgotPasswordRequest>,
) -> ApiResult {
    if let Some(user) = users::Model::find_by_email(&ctx.db, &normalize_email(&req.email)).await? {
        if user.is_verified() {
            let (user, token) = user
                .into_active_model()
                .set_forgot_password_sent(&ctx.db)
                .await?;
            RelayMessage {
                channel: "email".to_string(),
                recipient: user.email.clone(),
                template: "auth.reset_password".to_string(),
                tenant_id: None,
                data: json!({
                    "name": user.name,
                    "reset_url": format!(
                        "{}?token={token}",
                        Settings::load(&ctx)?.url("reset-password")
                    ),
                }),
            }
            .send(&ctx)
            .await?;
        }
    }
    Ok(StatusCode::ACCEPTED.into_response())
}

/// `PUT /v1/auth/reset-password`: set a new password with the emailed token.
/// The token is single use and every session of the user is revoked.
///
/// # Errors
///
/// `400` for an invalid token and `422` for an expired one. Fails too when the
/// new password cannot be saved.
#[debug_handler]
pub async fn reset_password(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<ResetPasswordRequest>,
) -> ApiResult {
    let user = users::Model::find_by_reset_token(&ctx.db, &req.token)
        .await?
        .ok_or_else(|| Problem::bad_request("invalid_token", "This reset link is not valid."))?;
    if user.reset_expired() {
        return Err(Problem::unprocessable(
            "token_expired",
            "This reset link has expired.",
        ));
    }

    user.into_active_model()
        .reset_password(&ctx.db, &req.new_password)
        .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
