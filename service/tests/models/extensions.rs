use std::collections::BTreeMap;

use loco_rs::testing::prelude::*;
use sea_orm::DatabaseConnection;
use serial_test::serial;
use zerpto_iam::{
    app::App,
    extensions::{manifest::Manifest, manifest_check::parse},
    models::{
        extensions::{self, ExtensionError},
        permissions::Catalogue,
        tenant_extensions,
        tenants::{self, NewTenant},
        users::{self, NewUser},
    },
};

const ADDRESS: &str = "https://example.com/manifest.yaml";

/// A manifest of `key` with one permission, requiring `services`.
fn manifest(key: &str, services: &[&str]) -> Manifest {
    parse(&format!(
        r#"
key: {key}
name: {key}
description: A test extension.
version: 1.0.0
core_api: "^1"
requires: {{ services: {services:?}, grpc: [], topics: [], permissions: [] }}
backend:
  publishes: []
  permissions:
    - {{ key: {key}.records.read, kind: read, minimum_plan: free, description: View records. }}
"#
    ))
    .expect("a readable manifest")
}

#[tokio::test]
#[serial]
async fn a_tenant_enables_dependencies_and_keeps_them() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;

    let user = users::Model::create_with_password(
        db,
        &NewUser {
            email: "owner@example.com",
            name: "Owner",
            password: "a-long-test-password",
            email_verified: true,
        },
    )
    .await
    .unwrap();
    // Created before any approval, so nothing is enabled for it.
    let tenant = tenants::Model::create_with_owner(
        db,
        user.id,
        NewTenant {
            name: "Acme".to_string(),
            legal_name: None,
            tax_id: None,
            base_currency: tenants::DEFAULT_CURRENCY.to_string(),
            timezone: tenants::DEFAULT_TIMEZONE.to_string(),
            fiscal_year_start_month: tenants::DEFAULT_FISCAL_YEAR_START_MONTH,
            completeness: tenants::SIMPLE.to_string(),
            address: None,
        },
    )
    .await
    .unwrap()
    .unwrap();

    let order_manifest = manifest("order", &["partner >=1.0"]);
    assert_eq!(
        extensions::Model::register(db, ADDRESS, &order_manifest)
            .await
            .unwrap()
            .unwrap_err(),
        ExtensionError::UnknownDependency("partner".to_string())
    );
    let partner = extensions::Model::register(db, ADDRESS, &manifest("partner", &[]))
        .await
        .unwrap()
        .unwrap();
    let order = extensions::Model::register(db, ADDRESS, &order_manifest)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        extensions::Model::approve(db, order.id).await.unwrap(),
        Err(ExtensionError::DependencyNotSatisfied(_))
    ));
    for id in [partner.id, order.id] {
        extensions::Model::approve(db, id).await.unwrap().unwrap();
    }

    let known = |catalogue: Catalogue| catalogue.is_known("order.records.read");
    assert!(!known(
        Catalogue::enabled(db, Some(tenant.id)).await.unwrap()
    ));

    let (_, also_enabled) = tenant_extensions::Model::enable(db, tenant.id, &order.api_id())
        .await
        .unwrap()
        .unwrap();
    assert_eq!(also_enabled, ["partner"]);
    assert!(known(
        Catalogue::enabled(db, Some(tenant.id)).await.unwrap()
    ));

    assert_eq!(
        tenant_extensions::Model::disable(db, tenant.id, &partner.api_id())
            .await
            .unwrap()
            .unwrap_err(),
        ExtensionError::Required("order".to_string())
    );
    for id in [order.api_id(), partner.api_id()] {
        tenant_extensions::Model::disable(db, tenant.id, &id)
            .await
            .unwrap()
            .unwrap();
    }
    assert!(tenant_extensions::Model::enabled_ids(db, tenant.id)
        .await
        .unwrap()
        .is_empty());
}

#[tokio::test]
#[serial]
async fn a_rejected_registration_goes_back_to_review_when_refreshed() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let wanted = manifest("order", &[]);
    let row = extensions::Model::register(db, ADDRESS, &wanted)
        .await
        .unwrap()
        .unwrap();

    let row = extensions::Model::reject(db, row.id, "Not yet.")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, extensions::REJECTED);
    assert_eq!(
        extensions::Model::approve(db, row.id)
            .await
            .unwrap()
            .unwrap_err(),
        ExtensionError::NothingToReview
    );

    let row = extensions::Model::refresh(db, row.id, &wanted)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.status, extensions::PENDING_REVIEW);
    let row = extensions::Model::approve(db, row.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.rejection_reason, None);

    // The same manifest again leaves nothing to review.
    let row = extensions::Model::refresh(db, row.id, &wanted)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.pending_manifest, None);

    let core = extensions::Model::find_by_key(db, "iam")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        extensions::Model::refresh(db, core.id, &manifest("iam", &[]))
            .await
            .unwrap()
            .unwrap_err(),
        ExtensionError::IsCore
    );
}

/// The `manifest_url` of the installed extension `key`.
async fn stored_address(db: &DatabaseConnection, key: &str) -> Option<String> {
    extensions::Model::find_by_key(db, key)
        .await
        .unwrap()
        .expect("the extension is installed")
        .manifest_url
}

#[tokio::test]
#[serial]
async fn a_core_extension_keeps_its_address() {
    let boot = boot_test::<App>().await.unwrap();
    let db = &boot.app_context.db;
    let relay = [manifest("relay", &[])];
    let addresses = BTreeMap::from([("relay".to_string(), ADDRESS.to_string())]);
    extensions::Model::install_core(db, &relay, &BTreeMap::new())
        .await
        .unwrap();
    assert_eq!(stored_address(db, "relay").await, None);

    // The same manifest, now with an address.
    extensions::Model::install_core(db, &relay, &addresses)
        .await
        .unwrap();
    assert_eq!(stored_address(db, "relay").await.as_deref(), Some(ADDRESS));

    // Waiting for a fetch: only the address changes.
    let moved = "https://relay.example.com/manifest.yaml".to_string();
    let addresses = BTreeMap::from([("relay".to_string(), moved.clone())]);
    extensions::Model::install_core(db, &[], &addresses)
        .await
        .unwrap();
    assert_eq!(stored_address(db, "relay").await, Some(moved));
}
