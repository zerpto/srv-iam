//! The manifests of the core extensions.
//!
//! Each extension's own project holds its `manifest.yaml` and serves it; `sync`
//! fetches it from the configured address. IAM's is also compiled in, so it exists
//! without a fetch.

use loco_rs::{Error, Result};
use serde_yaml_ng::Value;

use super::{
    manifest::Manifest,
    manifest_check::{self, ManifestError, CORE_KEYS},
};

const COMMUNITY: [&str; 1] = [
    // IAM's own, from the frontend in this repository. Its permissions must
    // exist even while that app is not running.
    include_str!("../../../federation/manifest.yaml"),
];

/// The fields of `frontend` that point at an app.
const APP_FIELDS: [&str; 4] = ["mode", "remote", "expose", "shared"];

/// The core manifests compiled into the service, kept in `ctx.shared_store`.
///
/// A public extension point: a service that extends this one inserts
/// `CoreManifests::community().with(...)` in `after_context` to add its own
/// core extensions.
#[derive(Debug, Clone)]
pub struct CoreManifests(Vec<&'static str>);

impl CoreManifests {
    /// The compiled-in core extensions of this edition.
    #[must_use]
    pub fn community() -> Self {
        Self(COMMUNITY.to_vec())
    }

    /// Adds the YAML of one more core manifest.
    #[must_use]
    pub fn with(mut self, yaml: &'static str) -> Self {
        self.0.push(yaml);
        self
    }

    /// The manifests, read from their YAML. One whose app address is still a
    /// `${name}` placeholder is read without the app.
    ///
    /// # Errors
    ///
    /// When one cannot be read.
    pub fn parse(&self) -> Result<Vec<Manifest>> {
        self.0
            .iter()
            .map(|yaml| {
                bundled(yaml).map_err(|err| Error::string(&format!("invalid core manifest: {err}")))
            })
            .collect()
    }
}

fn bundled(yaml: &str) -> Result<Manifest, serde_yaml_ng::Error> {
    let mut value: Value = serde_yaml_ng::from_str(yaml)?;
    if let Some(frontend) = value.get_mut("frontend").and_then(Value::as_mapping_mut) {
        let unfilled = frontend
            .get("remote")
            .and_then(Value::as_str)
            .is_some_and(|remote| remote.starts_with("${"));
        if unfilled {
            for field in APP_FIELDS {
                frontend.remove(field);
            }
        }
    }
    serde_yaml_ng::from_value(value)
}

/// Reads and checks the manifest fetched from the address of the core extension
/// `key`. It may not describe another extension.
///
/// # Errors
///
/// The first [`ManifestError`] found.
pub fn accept(key: &str, yaml: &str) -> Result<Manifest, ManifestError> {
    let manifest = manifest_check::parse(yaml)?;
    manifest_check::check(&manifest)?;
    if manifest.key != key || !CORE_KEYS.contains(&key) {
        return Err(ManifestError::Invalid(format!(
            "The address is configured for the core extension `{key}`, and the manifest is of `{}`.",
            manifest.key
        )));
    }
    Ok(manifest)
}
