//! Sessions and refresh tokens.
//!
//! Access tokens are stateless JWTs and are never stored. A session lives in
//! the cache (Redis in production) as `sess:<sid>`, holding the hash of its
//! one currently valid refresh token. Refresh tokens look like
//! `<sid>.<secret>` and are single use: rotating one parks the old hash under
//! `rt_used:<hash>`, so presenting it again is recognised as theft and kills
//! the session.

use std::time::Duration;

use chrono::Utc;
use loco_rs::{auth::jwt, hash::random_string, prelude::*};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map};
use uuid::Uuid;

use super::{digest::sha256_hex, memberships, scope::Scope, users};

/// Access token lifetime in seconds.
pub const ACCESS_TOKEN_TTL: u64 = 900;
const SESSION_TTL: Duration = Duration::from_hours(720);
const REFRESH_SECRET_LENGTH: usize = 48;

/// The cached state of one session. Ids are kept as text so the record does
/// not depend on how `Uuid` serializes.
#[derive(Debug, Serialize, Deserialize)]
struct SessionRecord {
    user_id: String,
    /// `None` before onboarding, when the user belongs to no tenant yet.
    tenant_id: Option<String>,
    /// Milliseconds since the epoch; compared with `users.sessions_valid_after`.
    created_at: i64,
    current_hash: String,
}

/// The tokens a session hands out.
#[derive(Debug)]
pub struct IssuedTokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Access token lifetime in seconds.
    pub expires_in: u64,
}

/// Why a refresh token was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum RefreshError {
    /// Unknown, expired, already used, or issued before a password change.
    Invalid,
}

fn session_key(sid: &str) -> String {
    format!("sess:{sid}")
}

fn used_key(hash: &str) -> String {
    format!("rt_used:{hash}")
}

fn now_millis() -> i64 {
    Utc::now().timestamp_millis()
}

fn cache_err(err: impl std::fmt::Display) -> Error {
    Error::string(&format!("session cache: {err}"))
}

fn access_token(
    ctx: &AppContext,
    user: &users::Model,
    scope: Option<&Scope>,
    sid: &str,
) -> Result<String> {
    let mut claims: Map<String, serde_json::Value> = Map::new();
    claims.insert("sid".to_string(), json!(sid));
    // A session without a tenant (before onboarding) carries no tenant claims.
    if let Some(scope) = scope {
        claims.insert("tenant_id".to_string(), json!(scope.tenant.api_id()));
        claims.insert(
            "roles".to_string(),
            json!(scope
                .roles
                .iter()
                .map(super::roles::Model::api_id)
                .collect::<Vec<_>>()),
        );
    }
    let secret = &ctx.config.get_jwt_config()?.secret;
    // `pid` is the user's API id: zerpto-core's `CurrentSession` reads it back
    // as `user_id()`.
    jwt::JWT::new(secret)
        .generate_token(ACCESS_TOKEN_TTL, user.api_id(), claims)
        .map_err(Error::wrap)
}

async fn save(ctx: &AppContext, sid: &str, record: &SessionRecord) -> Result<()> {
    ctx.cache
        .insert_with_expiry(&session_key(sid), record, SESSION_TTL)
        .await
        .map_err(cache_err)
}

async fn issue(
    ctx: &AppContext,
    user: &users::Model,
    scope: Option<&Scope>,
    sid: &str,
    created_at: i64,
) -> Result<IssuedTokens> {
    let refresh_token = format!("{sid}.{}", random_string(REFRESH_SECRET_LENGTH));
    save(
        ctx,
        sid,
        &SessionRecord {
            user_id: user.id.to_string(),
            tenant_id: scope.map(|s| s.tenant.id.to_string()),
            created_at,
            current_hash: sha256_hex(&refresh_token),
        },
    )
    .await?;
    Ok(IssuedTokens {
        access_token: access_token(ctx, user, scope, sid)?,
        refresh_token,
        expires_in: ACCESS_TOKEN_TTL,
    })
}

/// Starts a session for `user` in `scope` (`None` when the user has no tenant
/// yet) and returns its first tokens.
///
/// # Errors
///
/// When the session cannot be stored or the JWT cannot be signed.
pub async fn start(
    ctx: &AppContext,
    user: &users::Model,
    scope: Option<&Scope>,
) -> Result<IssuedTokens> {
    issue(ctx, user, scope, &Uuid::new_v4().to_string(), now_millis()).await
}

/// Ends a session: its refresh token stops working. Idempotent.
///
/// # Errors
///
/// When the cache is unreachable.
pub async fn revoke(ctx: &AppContext, sid: &str) -> Result<()> {
    ctx.cache.remove(&session_key(sid)).await.map_err(cache_err)
}

/// Keeps a session alive across a password change, which refuses every
/// session created before it: stamps the session as created now.
///
/// # Errors
///
/// When the cache is unreachable.
pub async fn renew(ctx: &AppContext, sid: &str) -> Result<()> {
    let key = session_key(sid);
    if let Some(mut record) = ctx
        .cache
        .get::<SessionRecord>(&key)
        .await
        .map_err(cache_err)?
    {
        record.created_at = now_millis();
        save(ctx, sid, &record).await?;
    }
    Ok(())
}

/// Exchanges a refresh token for new tokens, invalidating the old one.
///
/// A token that was already rotated away is treated as stolen: the whole
/// session is revoked.
///
/// # Errors
///
/// `Ok(Err(RefreshError::Invalid))` when the token is refused; the outer error
/// is an infrastructure failure.
pub async fn rotate(
    ctx: &AppContext,
    refresh_token: &str,
) -> Result<std::result::Result<IssuedTokens, RefreshError>> {
    let Some((sid, _)) = refresh_token.split_once('.') else {
        return Ok(Err(RefreshError::Invalid));
    };
    let hash = sha256_hex(refresh_token);

    let record = ctx
        .cache
        .get::<SessionRecord>(&session_key(sid))
        .await
        .map_err(cache_err)?;
    let Some(record) = record.filter(|r| r.current_hash == hash) else {
        if ctx
            .cache
            .contains_key(&used_key(&hash))
            .await
            .map_err(cache_err)?
        {
            revoke(ctx, sid).await?;
        }
        return Ok(Err(RefreshError::Invalid));
    };

    let user = match Uuid::parse_str(&record.user_id).ok() {
        Some(id) => users::Model::find_by_id(&ctx.db, id).await?,
        None => None,
    };
    let Some(user) = user else {
        revoke(ctx, sid).await?;
        return Ok(Err(RefreshError::Invalid));
    };

    // A tenant-scoped session ends when the user leaves that tenant.
    let scope = match record.tenant_id.as_deref() {
        None => None,
        Some(tenant_id) => {
            let membership = match Uuid::parse_str(tenant_id).ok() {
                Some(id) => memberships::Model::find_in_tenant(&ctx.db, user.id, id)
                    .await?
                    .filter(memberships::Model::is_active),
                None => None,
            };
            if let Some(membership) = membership {
                Some(Scope::load(&ctx.db, membership).await?)
            } else {
                revoke(ctx, sid).await?;
                return Ok(Err(RefreshError::Invalid));
            }
        }
    };
    if user
        .sessions_valid_after
        .is_some_and(|after| record.created_at < after.timestamp_millis())
    {
        revoke(ctx, sid).await?;
        return Ok(Err(RefreshError::Invalid));
    }

    ctx.cache
        .insert_with_expiry(&used_key(&hash), &sid.to_string(), SESSION_TTL)
        .await
        .map_err(cache_err)?;
    Ok(Ok(issue(
        ctx,
        &user,
        scope.as_ref(),
        sid,
        record.created_at,
    )
    .await?))
}
