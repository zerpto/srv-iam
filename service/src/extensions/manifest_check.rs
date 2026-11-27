//! Reading a `manifest.yaml` and checking everything that needs no database.

use std::collections::HashSet;

use semver::{Version, VersionReq};

use super::manifest::{Frontend, LocalizedText, Manifest, MenuItem, SettingsMenuItem};

/// The version of the core API that `core_api` ranges are matched against.
pub const CORE_API_VERSION: Version = Version::new(1, 0, 0);

/// The extensions installed with the platform. Their keys cannot be
/// registered. `billing` is reserved here although this edition does not ship
/// it: a service that extends this one adds it.
pub const CORE_KEYS: [&str; 4] = ["iam", "reference-data", "billing", "relay"];

/// The sidebar groups the core owns.
pub const GROUPS: [&str; 6] = [
    "sales",
    "purchases",
    "inventory",
    "accounting",
    "tax-assets",
    "catalog",
];

/// The groups of the tenant settings menu the core owns.
pub const SETTINGS_GROUPS: [&str; 4] = ["organization", "finance", "developers", "billing"];

/// Why a manifest was refused. Each carries a sentence for the operator.
#[derive(Debug, PartialEq, Eq)]
pub enum ManifestError {
    Invalid(String),
    CoreApiUnsupported(String),
    /// A permission, menu key or topic outside the extension's own name space,
    /// or declared twice.
    KeyCollision(String),
}

type Outcome = Result<(), ManifestError>;

fn invalid(message: impl Into<String>) -> ManifestError {
    ManifestError::Invalid(message.into())
}

/// A slug such as `order` or `sales-orders`.
fn is_slug(value: &str) -> bool {
    value.starts_with(|c: char| c.is_ascii_lowercase())
        && value
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// `parts` slugs joined by dots.
fn is_dotted(value: &str, parts: usize) -> bool {
    value.split('.').count() == parts && value.split('.').all(is_slug)
}

/// `zerpto.<service>.<entity>.<event>`.
fn is_topic(value: &str) -> bool {
    let parts: Vec<&str> = value.split('.').collect();
    parts.len() == 4
        && parts[0] == "zerpto"
        && parts[1..].iter().all(|part| {
            !part.is_empty()
                && part
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
        })
}

/// Reads the YAML of a manifest. It is not checked yet: see [`check`].
///
/// # Errors
///
/// `Invalid` when it is not YAML of the manifest's shape.
pub fn parse(yaml: &str) -> Result<Manifest, ManifestError> {
    serde_yaml_ng::from_str(yaml)
        .map_err(|err| invalid(format!("The manifest could not be read: {err}.")))
}

/// One entry of `requires.services` as the key and the version range.
///
/// # Errors
///
/// `Invalid` when it is not `<key> <range>`.
pub fn requirement(entry: &str) -> Result<(&str, VersionReq), ManifestError> {
    let bad = || {
        invalid(format!(
            "`{entry}` in `requires.services` must be like `partner >=1.0`."
        ))
    };
    let (key, range) = entry
        .trim()
        .split_once(char::is_whitespace)
        .ok_or_else(bad)?;
    if !is_slug(key) {
        return Err(bad());
    }
    Ok((key, VersionReq::parse(range.trim()).map_err(|_| bad())?))
}

fn check_header(manifest: &Manifest) -> Outcome {
    if !is_slug(&manifest.key) {
        return Err(invalid(
            "`key` must be lowercase letters, digits and dashes, starting with a letter.",
        ));
    }
    if manifest.name.trim().is_empty() || manifest.description.trim().is_empty() {
        return Err(invalid("`name` and `description` are required."));
    }
    if Version::parse(&manifest.version).is_err() {
        return Err(invalid(
            "`version` must be a semantic version, such as 1.2.0.",
        ));
    }
    let range = VersionReq::parse(&manifest.core_api)
        .map_err(|_| invalid("`core_api` must be a version range, such as ^1."))?;
    if !range.matches(&CORE_API_VERSION) {
        return Err(ManifestError::CoreApiUnsupported(format!(
            "The extension needs core API `{}`, and this platform has {CORE_API_VERSION}.",
            manifest.core_api
        )));
    }
    Ok(())
}

fn check_requires(manifest: &Manifest) -> Outcome {
    let mut seen = HashSet::new();
    for entry in &manifest.requires.services {
        let (key, _) = requirement(entry)?;
        if key == manifest.key || CORE_KEYS.contains(&key) || !seen.insert(key) {
            return Err(invalid(format!(
                "`requires.services` cannot list `{key}`: it is a core extension, the extension itself or listed twice."
            )));
        }
    }
    if let Some(topic) = manifest.requires.topics.iter().find(|t| !is_topic(t)) {
        return Err(invalid(format!("`{topic}` is not a topic name.")));
    }
    if let Some(key) = manifest
        .requires
        .permissions
        .iter()
        .find(|key| !is_dotted(key, 3))
    {
        return Err(invalid(format!("`{key}` is not a permission key.")));
    }
    Ok(())
}

/// A name the extension declares must be in its own name space and only once.
fn own<'a>(seen: &mut HashSet<&'a str>, name: &'a str, prefix: &str) -> Outcome {
    if !name.starts_with(prefix) {
        return Err(ManifestError::KeyCollision(format!(
            "`{name}` must start with `{prefix}`."
        )));
    }
    if !seen.insert(name) {
        return Err(ManifestError::KeyCollision(format!(
            "`{name}` is declared twice."
        )));
    }
    Ok(())
}

fn check_backend(manifest: &Manifest) -> Outcome {
    let permission_prefix = format!("{}.", manifest.key);
    let mut permissions = HashSet::new();
    for permission in &manifest.backend.permissions {
        if !is_dotted(&permission.key, 3) {
            return Err(invalid(format!(
                "`{}` must be `{{extension}}.{{resource}}.{{action}}`.",
                permission.key
            )));
        }
        if permission.description.trim().is_empty() {
            return Err(invalid(format!(
                "`{}` needs a description.",
                permission.key
            )));
        }
        own(&mut permissions, &permission.key, &permission_prefix)?;
    }
    let topic_prefix = format!("zerpto.{}.", manifest.key);
    let mut topics = HashSet::new();
    for topic in &manifest.backend.publishes {
        if !is_topic(topic) {
            return Err(invalid(format!("`{topic}` is not a topic name.")));
        }
        own(&mut topics, topic, &topic_prefix)?;
    }
    Ok(())
}

fn check_appearance(key: &str, icon: &str, label: &LocalizedText) -> Outcome {
    if icon.trim().is_empty() || label.en.trim().is_empty() || label.id.trim().is_empty() {
        return Err(invalid(format!(
            "Menu entry `{key}` needs an `icon` and a label in `en` and `id`."
        )));
    }
    Ok(())
}

/// A `core` extension may leave out `group` (a top-level entry) and
/// `permission` (every signed-in user sees it).
fn check_menu_item(item: &MenuItem, core: bool) -> Outcome {
    let key = &item.key;
    if !is_dotted(key, 2) {
        return Err(invalid(format!(
            "Menu key `{key}` must be `{{extension}}.{{name}}`."
        )));
    }
    if !item
        .group
        .as_deref()
        .map_or(core, |group| GROUPS.contains(&group))
    {
        return Err(invalid(format!(
            "Menu entry `{key}` needs a `group`, one of {}.",
            GROUPS.join(", ")
        )));
    }
    if !item
        .permission
        .as_deref()
        .map_or(core, |permission| is_dotted(permission, 3))
    {
        return Err(invalid(format!(
            "Menu entry `{key}` needs a `permission` key."
        )));
    }
    check_appearance(key, &item.icon, &item.label)?;
    let routes = std::iter::once(&item.route).chain(item.create_route.as_ref());
    for route in routes {
        // `//host` would leave the app.
        if !route.starts_with('/') || route.starts_with("//") {
            return Err(invalid(format!(
                "Menu entry `{key}` has a route that is not an app path."
            )));
        }
    }
    Ok(())
}

fn check_settings_item(item: &SettingsMenuItem) -> Outcome {
    let key = &item.key;
    if !is_dotted(key, 2) {
        return Err(invalid(format!(
            "Settings menu key `{key}` must be `{{extension}}.{{name}}`."
        )));
    }
    if item
        .group
        .as_deref()
        .is_some_and(|group| !SETTINGS_GROUPS.contains(&group))
    {
        return Err(invalid(format!(
            "Settings entry `{key}` has an unknown `group`; use one of {}, or null.",
            SETTINGS_GROUPS.join(", ")
        )));
    }
    if !is_dotted(&item.permission, 3) {
        return Err(invalid(format!(
            "Settings entry `{key}` needs a `permission` key."
        )));
    }
    check_appearance(key, &item.icon, &item.label)?;
    if !item.route.starts_with("/settings/") {
        return Err(invalid(format!(
            "Settings entry `{key}` needs a route under `/settings/`."
        )));
    }
    Ok(())
}

/// The app the web app loads: complete, or (core extensions only) left out.
fn check_remote(key: &str, frontend: &Frontend) -> Outcome {
    let none = frontend.mode.is_none()
        && frontend.remote.is_none()
        && frontend.expose.is_none()
        && frontend.shared.is_none();
    if none && CORE_KEYS.contains(&key) {
        return Ok(());
    }
    let remote_is_url = frontend.remote.as_deref().is_some_and(|remote| {
        reqwest::Url::parse(remote).is_ok_and(|url| matches!(url.scheme(), "http" | "https"))
    });
    let exposes = frontend
        .expose
        .as_deref()
        .is_some_and(|expose| !expose.trim().is_empty());
    if frontend.mode.is_none() || frontend.shared.is_none() || !remote_is_url || !exposes {
        return Err(invalid(
            "`frontend` needs `mode`, an http(s) `remote` address, an `expose` name and `shared`.",
        ));
    }
    Ok(())
}

fn check_frontend(manifest: &Manifest) -> Outcome {
    let Some(frontend) = &manifest.frontend else {
        return Ok(());
    };
    check_remote(&manifest.key, frontend)?;
    let prefix = format!("{}.", manifest.key);
    // An entry key is used once across both menus.
    let mut keys = HashSet::new();
    let core = CORE_KEYS.contains(&manifest.key.as_str());
    for item in &frontend.menu {
        check_menu_item(item, core)?;
        own(&mut keys, &item.key, &prefix)?;
    }
    for item in &frontend.settings_menu {
        check_settings_item(item)?;
        own(&mut keys, &item.key, &prefix)?;
    }
    Ok(())
}

/// Checks a manifest on its own. Whether the extensions it requires exist is
/// the registry's to check.
///
/// # Errors
///
/// The first [`ManifestError`] found.
pub fn check(manifest: &Manifest) -> Outcome {
    check_header(manifest)?;
    check_requires(manifest)?;
    check_backend(manifest)?;
    check_frontend(manifest)
}
