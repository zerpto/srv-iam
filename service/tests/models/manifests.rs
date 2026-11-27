use zerpto_iam::extensions::{
    core_manifests::{accept, CoreManifests},
    fetch::{allowed_url, FetchError},
    manifest::{LocalizedText, Manifest, SettingsMenuItem},
    manifest_check::{check, parse, ManifestError},
};

pub const ORDER: &str = r#"
key: order
name: Orders
description: Sales orders and purchase orders.
version: 1.2.0
core_api: "^1"
requires: { services: [], grpc: [], topics: [], permissions: [] }
backend:
  publishes: [zerpto.order.order.confirmed]
  permissions:
    - { key: order.sales-orders.read, kind: read, minimum_plan: free, description: View sales orders. }
frontend:
  mode: module-federation
  remote: https://app.zerpto.com/extensions/order/remoteEntry.js
  expose: ./App
  shared: { react: "^19.0.0" }
  menu:
    - key: order.sales-orders
      group: sales
      order: 20
      icon: clipboard-document-check
      label: { en: Orders, id: Pesanan }
      route: /sales/orders
      permission: order.sales-orders.read
"#;

fn order() -> Manifest {
    parse(ORDER).expect("a readable manifest")
}

#[test]
fn a_valid_manifest_passes() {
    assert_eq!(check(&order()), Ok(()));
}

#[test]
fn the_core_manifests_are_valid() {
    for manifest in CoreManifests::community().parse().unwrap() {
        assert_eq!(check(&manifest), Ok(()), "{}", manifest.key);
    }
}

#[test]
fn an_unsupported_core_api_is_refused() {
    let mut manifest = order();
    manifest.core_api = "^2".to_string();
    assert!(matches!(
        check(&manifest),
        Err(ManifestError::CoreApiUnsupported(_))
    ));
}

#[test]
fn names_outside_the_extension_collide() {
    let mut permission = order();
    permission.backend.permissions[0].key = "iam.users.manage".to_string();
    let mut topic = order();
    topic.backend.publishes[0] = "zerpto.iam.user.created".to_string();
    for manifest in [permission, topic] {
        assert!(matches!(
            check(&manifest),
            Err(ManifestError::KeyCollision(_))
        ));
    }
}

#[test]
fn a_menu_entry_needs_a_shared_group() {
    let mut manifest = order();
    manifest.frontend.as_mut().unwrap().menu[0].group = None;
    assert!(matches!(check(&manifest), Err(ManifestError::Invalid(_))));
}

#[test]
fn a_menu_entry_needs_a_permission() {
    let mut manifest = order();
    manifest.frontend.as_mut().unwrap().menu[0].permission = None;
    assert!(matches!(check(&manifest), Err(ManifestError::Invalid(_))));
}

#[test]
fn the_iam_manifest_declares_no_sidebar_entries() {
    let iam = accept("iam", &served_iam()).unwrap();
    // Dashboard, approvals and reports are commented out until they have screens.
    assert!(iam.frontend.unwrap().menu.is_empty());
}

#[test]
fn only_https_or_allowed_hosts_are_fetched() {
    assert!(allowed_url("https://example.com/manifest.yaml", "").is_ok());
    assert!(allowed_url("http://order:4009/manifest.yaml", "order, partner").is_ok());
    for address in [
        "http://example.com/manifest.yaml",
        "file:///etc/passwd",
        "https://user:secret@example.com/manifest.yaml",
        "not a url",
    ] {
        assert_eq!(
            allowed_url(address, "order"),
            Err(FetchError::UrlNotAllowed),
            "{address}"
        );
    }
}

/// `federation/manifest.yaml` as its app serves it.
fn served_iam() -> String {
    include_str!("../../../federation/manifest.yaml")
        .replace("${remote}", "http://localhost:3001/mf/mf-manifest.json")
        .replace("${expose}", "./Routes")
        .replace("${shared}", r#"{"react":"^19.2.0"}"#)
}

#[test]
fn the_compiled_in_iam_manifest_has_no_app() {
    let manifests = CoreManifests::community().parse().unwrap();
    let iam = manifests.iter().find(|m| m.key == "iam").unwrap();
    let frontend = iam.frontend.as_ref().unwrap();
    assert!(frontend.mode.is_none() && frontend.remote.is_none());
    assert!(frontend.expose.is_none() && frontend.shared.is_none());
    assert!(!frontend.settings_menu.is_empty());
}

#[test]
fn a_fetched_core_manifest_must_be_the_one_configured() {
    let iam = accept("iam", &served_iam()).unwrap();
    assert!(iam.frontend.unwrap().remote.is_some());

    // Another core service cannot replace IAM's, nor can a non-core key pass.
    assert!(matches!(
        accept("reference-data", &served_iam()),
        Err(ManifestError::Invalid(_))
    ));
    assert!(matches!(
        accept("order", ORDER),
        Err(ManifestError::Invalid(_))
    ));
}

#[test]
fn only_a_core_extension_may_leave_out_its_app() {
    let mut manifest = order();
    let frontend = manifest.frontend.as_mut().unwrap();
    frontend.mode = None;
    frontend.remote = None;
    frontend.expose = None;
    frontend.shared = None;
    assert!(matches!(check(&manifest), Err(ManifestError::Invalid(_))));
}

#[test]
fn a_settings_entry_stays_under_settings_and_has_its_own_key() {
    let entry = SettingsMenuItem {
        key: "order.purchasing-settings".to_string(),
        group: Some("finance".to_string()),
        order: 20,
        icon: "adjustments-horizontal".to_string(),
        label: LocalizedText {
            en: "Purchasing".to_string(),
            id: "Pembelian".to_string(),
        },
        route: "/settings/purchasing".to_string(),
        permission: "order.purchasing-settings.read".to_string(),
    };
    let with = |entry: SettingsMenuItem| {
        let mut manifest = order();
        manifest.frontend.as_mut().unwrap().settings_menu = vec![entry];
        check(&manifest)
    };
    assert_eq!(with(entry.clone()), Ok(()));

    let outside = SettingsMenuItem {
        route: "/purchases/settings".to_string(),
        ..entry.clone()
    };
    assert!(matches!(with(outside), Err(ManifestError::Invalid(_))));

    // The same key as the sidebar entry.
    let taken = SettingsMenuItem {
        key: "order.sales-orders".to_string(),
        ..entry
    };
    assert!(matches!(with(taken), Err(ManifestError::KeyCollision(_))));
}
