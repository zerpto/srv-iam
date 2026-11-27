//! Downloading a `manifest.yaml`, only from addresses the installation allows.

use std::time::Duration;

use reqwest::{redirect::Policy, Client, Url};

const TIMEOUT: Duration = Duration::from_secs(10);
const MAX_BYTES: usize = 256 * 1024;

/// Why a manifest could not be downloaded.
#[derive(Debug, PartialEq, Eq)]
pub enum FetchError {
    UrlNotAllowed,
    /// With the cause, for the operator.
    Unreachable(String),
}

/// An HTTPS address, or an HTTP(S) address of one of `allowed_hosts` (host
/// names separated by commas). Credentials in the address are never allowed.
///
/// # Errors
///
/// `UrlNotAllowed`.
pub fn allowed_url(address: &str, allowed_hosts: &str) -> Result<Url, FetchError> {
    let url = Url::parse(address).map_err(|_| FetchError::UrlNotAllowed)?;
    if !url.username().is_empty() || url.password().is_some() {
        return Err(FetchError::UrlNotAllowed);
    }
    let host = url.host_str().ok_or(FetchError::UrlNotAllowed)?;
    let internal = allowed_hosts
        .split(',')
        .map(str::trim)
        .any(|allowed| !allowed.is_empty() && allowed.eq_ignore_ascii_case(host));
    match url.scheme() {
        "https" => Ok(url),
        "http" if internal => Ok(url),
        _ => Err(FetchError::UrlNotAllowed),
    }
}

fn unreachable(cause: impl std::fmt::Display) -> FetchError {
    FetchError::Unreachable(cause.to_string())
}

/// The text of the manifest at `address`. Redirects are not followed, so an
/// allowed address cannot lead to one that is not.
///
/// # Errors
///
/// `UrlNotAllowed`, or `Unreachable` for anything but a `2xx` answer with at
/// most 256 KiB of UTF-8.
pub async fn download(address: &str, allowed_hosts: &str) -> Result<String, FetchError> {
    let url = allowed_url(address, allowed_hosts)?;
    let client = Client::builder()
        .timeout(TIMEOUT)
        .redirect(Policy::none())
        .build()
        .map_err(unreachable)?;
    let mut response = client.get(url).send().await.map_err(unreachable)?;
    if !response.status().is_success() {
        return Err(unreachable(format!("it answered {}", response.status())));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(unreachable)? {
        if body.len() + chunk.len() > MAX_BYTES {
            return Err(unreachable("the file is larger than 256 KiB"));
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| unreachable("the file is not UTF-8 text"))
}
