use axum::http::StatusCode;
use loco_rs::prelude::*;
use serde_json::json;
use uuid::Uuid;
use zerpto_core::{
    helpers::json,
    http::problem::{ApiResult, Problem},
    relay::message::RelayMessage,
    validation::ValidatedJson,
};

use super::auth::signed_in;
use crate::{
    google::{self, GoogleError},
    http::{
        auth_session::AuthSession,
        v1::{
            requests::verify::{
                GoogleSignInRequest, PhoneCodeRequest, PhoneVerifyRequest, VerifyEmailRequest,
            },
            response::{auth::AuthAccount, verify::VerifyEmailMeta},
        },
    },
    models::{
        phone_verifications::{self, SendError, VerifyError},
        users::{self, NewUser},
    },
    settings::Settings,
};

/// `POST /v1/auth/verify-email`: confirm the address with the emailed token.
/// Repeating it for an already confirmed address is a no-op.
///
/// # Errors
///
/// `400` for an invalid token and `422` for an expired one. Fails too when the
/// confirmation cannot be saved.
#[debug_handler]
pub async fn verify_email(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<VerifyEmailRequest>,
) -> ApiResult {
    let user = users::Model::find_by_verification_token(&ctx.db, &req.token)
        .await?
        .ok_or_else(|| {
            Problem::bad_request("invalid_token", "This verification link is not valid.")
        })?;

    // An address that is already confirmed answers like the first time.
    let user = if user.is_verified() {
        user
    } else if user.verification_expired() {
        return Err(Problem::unprocessable(
            "token_expired",
            "This verification link has expired.",
        ));
    } else {
        user.into_active_model().verified(&ctx.db).await?
    };

    Ok(json(
        StatusCode::OK,
        AuthAccount::from(&user),
        VerifyEmailMeta {
            web_urls: vec![Settings::load(&ctx)?.url("login")],
        },
    ))
}

/// `POST /v1/auth/google`: exchange a Google ID token for tokens, creating the
/// account on the first sign-in. Google vouches for the email, so no
/// confirmation step follows.
///
/// # Errors
///
/// `401` for an invalid Google token or an email Google has not verified. Fails
/// too when the token cannot be checked or the account cannot be saved.
#[debug_handler]
pub async fn google(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<GoogleSignInRequest>,
) -> ApiResult {
    let settings = Settings::load(&ctx)?.google;
    let identity = google::verify_id_token(
        &settings.tokeninfo_url,
        &settings.client_id,
        req.id_token.trim(),
    )
    .await?
    .map_err(|err| match err {
        GoogleError::InvalidToken => {
            Problem::unauthorized("invalid_google_token", "This Google sign-in is not valid.")
        }
        GoogleError::EmailNotVerified => Problem::unauthorized(
            "google_email_not_verified",
            "Google has not verified this email address.",
        ),
    })?;

    let user = match users::Model::find_by_email(&ctx.db, &identity.email).await? {
        Some(user) if user.is_verified() => user,
        Some(user) => user.into_active_model().verified(&ctx.db).await?,
        None => {
            // The account has no password anyone knows: it is a random one.
            let password = format!("{}{}", Uuid::new_v4(), Uuid::new_v4());
            match users::Model::create_with_password(
                &ctx.db,
                &NewUser {
                    email: &identity.email,
                    name: &identity.name,
                    password: &password,
                    email_verified: true,
                },
            )
            .await
            {
                Ok(user) => user,
                // Two first sign-ins at once: the other one created it.
                Err(ModelError::EntityAlreadyExists) => {
                    users::Model::find_by_email(&ctx.db, &identity.email)
                        .await?
                        .ok_or_else(|| Problem::from(Error::string("google user vanished")))?
                }
                Err(err) => return Err(err.into()),
            }
        }
    };
    signed_in(&ctx, &user).await
}

/// `POST /v1/auth/phone/code`: send a 6-digit code to a phone number.
///
/// # Errors
///
/// `400` for a malformed phone number and `429` after too many codes. Fails too
/// when the code cannot be saved or the message cannot be queued.
#[debug_handler]
pub async fn send_phone_code(
    State(ctx): State<AppContext>,
    session: AuthSession,
    ValidatedJson(body): ValidatedJson<PhoneCodeRequest>,
) -> ApiResult {
    let user = session.user(&ctx).await?;
    let secret = ctx.config.get_jwt_config()?.secret.clone();
    let settings = Settings::load(&ctx)?;
    let code = phone_verifications::Model::issue(
        &ctx.db,
        user.id,
        &body.phone,
        &body.channel,
        &secret,
        &settings.phone,
        chrono::Utc::now(),
    )
    .await?
    .map_err(|err| match err {
        SendError::InvalidPhone => Problem::bad_request(
            "invalid_phone",
            "Use the international form with the country code, such as +6281234567890.",
        )
        .field("phone"),
        SendError::TooManyCodes => Problem::too_many_requests(
            "too_many_codes",
            "Too many codes were asked for. Try again in an hour.",
        ),
    })?;

    // The code is only ever sent to the number it is for.
    let phone = phone_verifications::normalize(&body.phone).unwrap_or_default();
    RelayMessage {
        channel: body.channel,
        recipient: phone,
        template: "auth.phone_code".to_string(),
        tenant_id: None,
        data: json!({ "code": code, "ttl_minutes": settings.phone.code_ttl_minutes }),
    }
    .send(&ctx)
    .await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `POST /v1/auth/phone/verify`: check the code that was sent.
///
/// # Errors
///
/// `422` for a wrong or expired code and `429` after too many wrong guesses.
/// Fails too when the result cannot be saved.
#[debug_handler]
pub async fn verify_phone_code(
    State(ctx): State<AppContext>,
    session: AuthSession,
    ValidatedJson(body): ValidatedJson<PhoneVerifyRequest>,
) -> ApiResult {
    let user = session.user(&ctx).await?;
    let secret = ctx.config.get_jwt_config()?.secret.clone();
    let settings = Settings::load(&ctx)?;
    phone_verifications::Model::verify(
        &ctx.db,
        &user,
        &body.phone,
        &body.code,
        &secret,
        &settings.phone,
        chrono::Utc::now(),
    )
    .await?
    .map_err(|err| match err {
        VerifyError::InvalidCode => {
            Problem::unprocessable("invalid_code", "The code is not right.").field("code")
        }
        VerifyError::CodeExpired => {
            Problem::unprocessable("code_expired", "The code has expired. Ask for a new one.")
                .field("code")
        }
        VerifyError::TooManyAttempts => Problem::too_many_requests(
            "too_many_attempts",
            "Too many wrong codes. Ask for a new one.",
        ),
    })?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
