use loco_rs::prelude::*;
use serde::Deserialize;

/// The `iss` values Google puts in an ID token.
const ISSUERS: [&str; 2] = ["accounts.google.com", "https://accounts.google.com"];

/// Why an ID token was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum GoogleError {
    /// Not a token Google issued for this app, or expired.
    InvalidToken,
    /// Google has not verified the email address of the token.
    EmailNotVerified,
}

/// Who the token says is signing in.
#[derive(Debug, PartialEq, Eq)]
pub struct GoogleIdentity {
    pub email: String,
    pub name: String,
}

/// Google's answer to `tokeninfo`; every value is a string.
#[derive(Deserialize)]
struct TokenInfo {
    iss: String,
    aud: String,
    exp: String,
    email: Option<String>,
    email_verified: Option<String>,
    name: Option<String>,
}

/// Has Google check `id_token` and returns the person it belongs to.
///
/// `client_ids` are the OAuth client IDs of this app, separated by commas:
/// the token must have been issued for one of them.
///
/// # Errors
///
/// `Ok(Err(..))` says why the token was refused; the outer error is a failed
/// call to Google.
pub async fn verify_id_token(
    tokeninfo_url: &str,
    client_ids: &str,
    id_token: &str,
) -> Result<std::result::Result<GoogleIdentity, GoogleError>> {
    let response = reqwest::Client::new()
        .get(tokeninfo_url)
        .query(&[("id_token", id_token)])
        .send()
        .await
        .map_err(Error::wrap)?;
    let status = response.status();
    if status.is_client_error() {
        return Ok(Err(GoogleError::InvalidToken));
    }
    if !status.is_success() {
        return Err(Error::string(&format!(
            "google tokeninfo answered {status}"
        )));
    }
    let info: TokenInfo = response.json().await.map_err(Error::wrap)?;
    Ok(check(&info, client_ids, chrono::Utc::now().timestamp()))
}

fn check(
    info: &TokenInfo,
    client_ids: &str,
    now: i64,
) -> std::result::Result<GoogleIdentity, GoogleError> {
    let for_this_app = client_ids
        .split(',')
        .map(str::trim)
        .any(|id| !id.is_empty() && id == info.aud);
    let unexpired = info.exp.parse::<i64>().is_ok_and(|exp| exp > now);
    if !for_this_app || !unexpired || !ISSUERS.contains(&info.iss.as_str()) {
        return Err(GoogleError::InvalidToken);
    }
    let email = info
        .email
        .as_deref()
        .map(|email| email.trim().to_lowercase())
        .filter(|email| !email.is_empty())
        .ok_or(GoogleError::InvalidToken)?;
    if info.email_verified.as_deref() != Some("true") {
        return Err(GoogleError::EmailNotVerified);
    }
    let name = info
        .name
        .as_deref()
        .map(str::trim)
        .filter(|name| name.chars().count() >= 2)
        .map_or_else(
            || email.split('@').next().unwrap_or_default().to_string(),
            str::to_string,
        );
    Ok(GoogleIdentity { email, name })
}
