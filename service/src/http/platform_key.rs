//! The platform operator's key (`X-Platform-Key`). It identifies who runs the
//! platform and is not a tenant role; only the extension registry accepts it.

use std::future::{self, Future};

use axum::{extract::FromRequestParts, http::request::Parts};
use loco_rs::app::AppContext;
use sha2::{Digest, Sha256};
use zerpto_core::http::problem::Problem;

use super::auth_session::AuthSession;
use crate::settings::Settings;

const HEADER: &str = "x-platform-key";

fn unauthorized() -> Problem {
    Problem::unauthorized("invalid_platform_key", "Missing or invalid platform key.")
}

/// A request from the platform operator.
#[derive(Debug)]
pub struct PlatformOperator;

fn check(parts: &Parts, ctx: &AppContext) -> Result<PlatformOperator, Problem> {
    let sent = parts.headers.get(HEADER).ok_or_else(unauthorized)?;
    let expected = Settings::load(ctx)?.extensions.platform_key;
    // An installation without a key refuses everyone. Digests are compared so
    // the time taken says nothing about the key.
    if expected.is_empty() || Sha256::digest(sent.as_bytes()) != Sha256::digest(expected.as_bytes())
    {
        return Err(unauthorized());
    }
    Ok(PlatformOperator)
}

impl FromRequestParts<AppContext> for PlatformOperator {
    type Rejection = Problem;

    fn from_request_parts(
        parts: &mut Parts,
        ctx: &AppContext,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        future::ready(check(parts, ctx))
    }
}

/// Who reads the registry: the operator when a platform key is sent,
/// otherwise a signed-in user.
#[derive(Debug)]
pub enum Viewer {
    Operator,
    User(AuthSession),
}

impl FromRequestParts<AppContext> for Viewer {
    type Rejection = Problem;

    async fn from_request_parts(
        parts: &mut Parts,
        ctx: &AppContext,
    ) -> Result<Self, Self::Rejection> {
        if parts.headers.contains_key(HEADER) {
            check(parts, ctx)?;
            return Ok(Self::Operator);
        }
        Ok(Self::User(
            AuthSession::from_request_parts(parts, ctx).await?,
        ))
    }
}
