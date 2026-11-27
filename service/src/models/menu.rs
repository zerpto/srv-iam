//! The sidebar menu and the tenant settings menu: the core's groups, filled with
//! the entries of the active extensions' manifests that the person may see.
//!
//! The filtering is cosmetic; every service still checks permissions.

use crate::extensions::manifest::{LocalizedText, MenuItem, SettingsMenuItem};

/// A shared group with the entries the person sees in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    pub key: &'static str,
    pub order: i64,
    pub label: LocalizedText,
    pub items: Vec<MenuItem>,
}

/// A shared settings group with the entries the person sees in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsGroup {
    pub key: &'static str,
    pub order: i64,
    pub label: LocalizedText,
    pub items: Vec<SettingsMenuItem>,
}

/// The tenant settings menu, arranged like the sidebar on its own scale.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettingsMenu {
    pub items: Vec<SettingsMenuItem>,
    pub groups: Vec<SettingsGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Menu {
    /// Entries outside any group. Their `order` shares one scale with the groups.
    pub items: Vec<MenuItem>,
    pub groups: Vec<Group>,
    pub settings: SettingsMenu,
}

/// Key, position, English and Indonesian label.
const GROUPS: [(&str, i64, &str, &str); 6] = [
    ("sales", 100, "Sales", "Penjualan"),
    ("purchases", 200, "Purchases", "Pembelian"),
    ("inventory", 300, "Inventory", "Persediaan"),
    ("accounting", 400, "Accounting", "Akuntansi"),
    ("tax-assets", 500, "Tax & assets", "Pajak & aset"),
    ("catalog", 700, "Catalog", "Katalog"),
];

/// Key, position, English and Indonesian label.
const SETTINGS_GROUPS: [(&str, i64, &str, &str); 4] = [
    ("organization", 100, "Organization", "Organisasi"),
    ("finance", 200, "Finance", "Keuangan"),
    ("developers", 300, "Developers", "Pengembang"),
    ("billing", 400, "Billing", "Tagihan"),
];

fn text(en: &str, id: &str) -> LocalizedText {
    LocalizedText {
        en: en.to_string(),
        id: id.to_string(),
    }
}

fn sort(items: &mut [MenuItem]) {
    items.sort_by(|a, b| (a.order, &a.key).cmp(&(b.order, &b.key)));
}

/// The settings entries the person may see, in their groups. Groups with
/// nothing to show are left out.
fn settings_menu(mut entries: Vec<SettingsMenuItem>, allowed: &[String]) -> SettingsMenu {
    entries.retain(|item| allowed.contains(&item.permission));
    entries.sort_by(|a, b| (a.order, &a.key).cmp(&(b.order, &b.key)));
    let groups = SETTINGS_GROUPS
        .into_iter()
        .filter_map(|(key, order, en, id)| {
            let items: Vec<SettingsMenuItem> = entries
                .iter()
                .filter(|item| item.group.as_deref() == Some(key))
                .cloned()
                .collect();
            (!items.is_empty()).then(|| SettingsGroup {
                key,
                order,
                label: text(en, id),
                items,
            })
        })
        .collect();
    entries.retain(|item| item.group.is_none());
    SettingsMenu {
        items: entries,
        groups,
    }
}

/// Builds both menus from the sidebar `entries` and the `settings` entries of the
/// active extensions.
///
/// `allowed` is what the person may really do (for a `read_only` person it holds no
/// write key); a `read_only` person also gets no shortcut to create a record.
/// Groups with nothing to show are left out.
#[must_use]
pub fn build(
    entries: Vec<MenuItem>,
    settings: Vec<SettingsMenuItem>,
    allowed: &[String],
    read_only: bool,
) -> Menu {
    let mut visible: Vec<MenuItem> = entries
        .into_iter()
        .filter(|item| {
            item.permission
                .as_ref()
                .is_none_or(|permission| allowed.contains(permission))
        })
        .collect();
    if read_only {
        for item in &mut visible {
            item.create_route = None;
        }
    }
    sort(&mut visible);
    let groups = GROUPS
        .into_iter()
        .filter_map(|(key, order, en, id)| {
            let items: Vec<MenuItem> = visible
                .iter()
                .filter(|item| item.group.as_deref() == Some(key))
                .cloned()
                .collect();
            (!items.is_empty()).then(|| Group {
                key,
                order,
                label: text(en, id),
                items,
            })
        })
        .collect();
    visible.retain(|item| item.group.is_none());
    Menu {
        items: visible,
        groups,
        settings: settings_menu(settings, allowed),
    }
}
