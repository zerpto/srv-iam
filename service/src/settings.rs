use std::collections::BTreeMap;

use loco_rs::prelude::*;
use serde::Deserialize;

/// This service's own part of the `settings:` block of `config/*.yaml`.
/// Shared parts (gRPC, events, storage) are
/// `zerpto_core::settings::CoreSettings`, read from the same block.
#[derive(Debug, Deserialize)]
pub struct Settings {
    /// The web app that email links and `web_urls` point to.
    pub web_app_url: String,
    /// The reference-data service's gRPC address (`REFERENCE_DATA_GRPC_URL`),
    /// asked to check a tenant's address. Empty means it is not configured,
    /// and an address cannot be checked (the request fails with `503`).
    #[serde(default)]
    pub reference_data_url: String,
    /// Verification codes for phone numbers.
    #[serde(default)]
    pub phone: PhoneSettings,
    /// Signing in with Google.
    #[serde(default)]
    pub google: GoogleSettings,
    /// The extension registry.
    #[serde(default)]
    pub extensions: ExtensionSettings,
}

/// The extension registry.
#[derive(Debug, Default, Deserialize)]
pub struct ExtensionSettings {
    /// The platform operator's key, sent as `X-Platform-Key`
    /// (`PLATFORM_API_KEY`). Left empty, every operator call is refused.
    #[serde(default)]
    pub platform_key: String,
    /// Internal host names a manifest may be fetched from over plain HTTP,
    /// separated by commas (`EXTENSION_ALLOWED_HOSTS`). HTTPS addresses need
    /// no entry.
    #[serde(default)]
    pub allowed_hosts: String,
    /// Where each core extension serves its `manifest.yaml`, by extension key
    /// (`IAM_MANIFEST_URL`, `REFERENCE_DATA_MANIFEST_URL`,
    /// `RELAY_MANIFEST_URL`, `BILLING_MANIFEST_URL`), stored as the extension's
    /// `manifest_url`. An empty address is not fetched. A plain-HTTP host must be in `allowed_hosts`.
    #[serde(default)]
    pub core_manifest_urls: BTreeMap<String, String>,
}

/// Signing in with Google.
#[derive(Debug, Deserialize)]
pub struct GoogleSettings {
    /// OAuth client IDs of the apps allowed to sign in (web, iOS, Android),
    /// separated by commas (`GOOGLE_CLIENT_ID`). Left empty, every Google
    /// sign-in is refused.
    #[serde(default)]
    pub client_id: String,
    /// Google's token check; only tests change it.
    #[serde(default = "default_google_tokeninfo_url")]
    pub tokeninfo_url: String,
}

impl Default for GoogleSettings {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            tokeninfo_url: default_google_tokeninfo_url(),
        }
    }
}

fn default_google_tokeninfo_url() -> String {
    "https://oauth2.googleapis.com/tokeninfo".to_string()
}

/// Limits on phone verification codes.
#[derive(Debug, Deserialize)]
pub struct PhoneSettings {
    /// Minutes a code stays valid.
    #[serde(default = "default_code_ttl_minutes")]
    pub code_ttl_minutes: i64,
    /// Wrong guesses allowed per code.
    #[serde(default = "default_max_attempts")]
    pub max_attempts: i32,
    /// Codes one person, or one number, may ask for per hour.
    #[serde(default = "default_max_sends_per_hour")]
    pub max_sends_per_hour: i32,
}

const fn default_code_ttl_minutes() -> i64 {
    5
}

const fn default_max_attempts() -> i32 {
    5
}

const fn default_max_sends_per_hour() -> i32 {
    5
}

impl Default for PhoneSettings {
    fn default() -> Self {
        Self {
            code_ttl_minutes: default_code_ttl_minutes(),
            max_attempts: default_max_attempts(),
            max_sends_per_hour: default_max_sends_per_hour(),
        }
    }
}

impl Settings {
    /// # Errors
    ///
    /// When the `settings:` block is missing or malformed.
    pub fn load(ctx: &AppContext) -> Result<Self> {
        ctx.config.settings()
    }

    /// A page of the web app, e.g. `url("login")`.
    #[must_use]
    pub fn url(&self, path: &str) -> String {
        format!("{}/{path}", self.web_app_url.trim_end_matches('/'))
    }
}
