//! The `manifest.yaml` an extension serves (`ExtensionManifest` in the API
//! contract). The same shape is stored as JSON and sent in responses.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(rename = "ExtensionManifest")]
pub struct Manifest {
    pub key: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub core_api: String,
    pub requires: Requires,
    pub backend: Backend,
    /// `None` when the extension has no screens.
    #[serde(default)]
    pub frontend: Option<Frontend>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(rename = "ManifestRequires")]
pub struct Requires {
    /// Non-core extensions, each as `<key> <version range>`.
    pub services: Vec<String>,
    pub grpc: Vec<String>,
    pub topics: Vec<String>,
    pub permissions: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(rename = "ManifestBackend")]
pub struct Backend {
    pub publishes: Vec<String>,
    pub permissions: Vec<ManifestPermission>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct ManifestPermission {
    pub key: String,
    pub kind: Kind,
    pub minimum_plan: MinimumPlan,
    pub description: String,
}

/// `read` only views data; `write` creates or changes it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Read,
    Write,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
pub enum MinimumPlan {
    Free,
    Paid,
}

/// The web part of an extension.
///
/// `mode`, `remote`, `expose` and `shared` describe the app the web app loads: a
/// registered extension needs all four, while a core one without an app leaves them
/// all out and only adds entries.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(rename = "ManifestFrontend")]
pub struct Frontend {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub mode: Option<Mode>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub remote: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub expose: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub shared: Option<BTreeMap<String, String>>,
    /// Entries of the sidebar groups.
    #[serde(default)]
    pub menu: Vec<MenuItem>,
    /// Entries of the tenant settings menu.
    #[serde(default)]
    pub settings_menu: Vec<SettingsMenuItem>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
pub enum Mode {
    ModuleFederation,
}

/// One sidebar entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct MenuItem {
    pub key: String,
    /// A shared group; `None` only for the core's top-level entries.
    #[serde(default)]
    pub group: Option<String>,
    #[ts(type = "number")]
    pub order: i64,
    pub icon: String,
    pub label: LocalizedText,
    pub route: String,
    /// `None` only for a core entry every signed-in user sees.
    #[serde(default)]
    pub permission: Option<String>,
    #[serde(default)]
    pub badge: Option<String>,
    #[serde(default)]
    pub create_route: Option<String>,
}

/// One entry of the tenant settings menu.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct SettingsMenuItem {
    pub key: String,
    /// A settings group of the core; `None` for an entry outside any group.
    #[serde(default)]
    pub group: Option<String>,
    #[ts(type = "number")]
    pub order: i64,
    pub icon: String,
    pub label: LocalizedText,
    pub route: String,
    pub permission: String,
}

/// A label in English (`en`) and Bahasa Indonesia (`id`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
pub struct LocalizedText {
    pub en: String,
    pub id: String,
}

impl Requires {
    /// The keys of the extensions this one depends on.
    #[must_use]
    pub fn service_keys(&self) -> Vec<&str> {
        self.services
            .iter()
            .filter_map(|service| service.split_whitespace().next())
            .collect()
    }
}
