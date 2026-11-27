use chrono::{DateTime, Utc};
use loco_rs::model::ModelResult;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::{
    extensions::manifest::{LocalizedText, Manifest, MenuItem, SettingsMenuItem},
    models::{extensions, menu},
};

/// An extension (`Extension` in the API contract).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Extension {
    pub id: String,
    pub key: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub party: String,
    pub core: bool,
    /// `pending_review`, `approved` or `rejected`.
    pub status: String,
    /// Whether the signed-in user's tenant uses it; `null` for the operator.
    pub enabled: Option<bool>,
    pub manifest_url: Option<String>,
    pub manifest: Option<Manifest>,
    pub pending_manifest: Option<Manifest>,
    pub rejection_reason: Option<String>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Extension {
    /// Everything, as the platform operator sees it.
    ///
    /// # Errors
    ///
    /// When a stored manifest cannot be read.
    pub fn for_operator(row: &extensions::Model) -> ModelResult<Self> {
        Ok(Self {
            id: row.api_id(),
            key: row.key.clone(),
            name: row.name.clone(),
            description: row.description.clone(),
            version: row.version.clone(),
            party: extensions::FIRST_PARTY.to_string(),
            core: row.core,
            status: row.status.clone(),
            enabled: None,
            manifest_url: row.manifest_url.clone(),
            manifest: row.live_manifest()?,
            pending_manifest: row.waiting_manifest()?,
            rejection_reason: row.rejection_reason.clone(),
            reviewed_at: row.reviewed_at.map(|at| at.with_timezone(&Utc)),
            created_at: row.created_at.with_timezone(&Utc),
            updated_at: row.updated_at.with_timezone(&Utc),
        })
    }

    /// As a signed-in user sees it: nothing of the review.
    ///
    /// # Errors
    ///
    /// As [`for_operator`](Self::for_operator).
    pub fn for_user(row: &extensions::Model, enabled: bool) -> ModelResult<Self> {
        Ok(Self {
            enabled: Some(enabled),
            manifest_url: None,
            pending_manifest: None,
            rejection_reason: None,
            ..Self::for_operator(row)?
        })
    }
}

/// `meta` of enabling an extension.
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct ExtensionEnableMeta {
    /// Keys of the dependencies the call switched on.
    pub also_enabled: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct MenuGroup {
    pub key: String,
    #[ts(type = "number")]
    pub order: i64,
    pub label: LocalizedText,
    pub items: Vec<MenuItem>,
}

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct SettingsGroup {
    pub key: String,
    #[ts(type = "number")]
    pub order: i64,
    pub label: LocalizedText,
    pub items: Vec<SettingsMenuItem>,
}

/// The tenant settings menu (`SettingsMenu`).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct SettingsMenu {
    pub items: Vec<SettingsMenuItem>,
    pub groups: Vec<SettingsGroup>,
}

impl From<menu::SettingsMenu> for SettingsMenu {
    fn from(menu: menu::SettingsMenu) -> Self {
        Self {
            items: menu.items,
            groups: menu
                .groups
                .into_iter()
                .map(|group| SettingsGroup {
                    key: group.key.to_string(),
                    order: group.order,
                    label: group.label,
                    items: group.items,
                })
                .collect(),
        }
    }
}

/// The sidebar and the settings menu (`Menu` in the API contract).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Menu {
    pub items: Vec<MenuItem>,
    pub groups: Vec<MenuGroup>,
    pub settings: SettingsMenu,
}

impl From<menu::Menu> for Menu {
    fn from(menu: menu::Menu) -> Self {
        Self {
            settings: menu.settings.into(),
            items: menu.items,
            groups: menu
                .groups
                .into_iter()
                .map(|group| MenuGroup {
                    key: group.key.to_string(),
                    order: group.order,
                    label: group.label,
                    items: group.items,
                })
                .collect(),
        }
    }
}
