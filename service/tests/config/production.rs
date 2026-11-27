use std::path::Path;

use loco_rs::{
    config::{CacheConfig, Config},
    environment::Environment,
};
use serial_test::serial;
use zerpto_core::{relay::RelaySettings, settings::CoreSettings};

/// The variables `config/production.yaml` requires.
const VARS: [(&str, &str); 16] = [
    ("DATABASE_URL", "postgres://user:pass@localhost/service"),
    ("JWT_SECRET", "test-secret"),
    ("HOST", "https://api.example.com"),
    ("REDIS_URL", "redis://127.0.0.1:6379"),
    ("NATS_URL", "nats://127.0.0.1:4222"),
    ("WEB_APP_URL", "https://app.example.com"),
    ("REFERENCE_DATA_GRPC_URL", "http://reference-data:5002"),
    ("API_URL", "https://api.example.com"),
    ("RELAY_REDIS_URL", "redis://127.0.0.1:6380"),
    ("R2_BUCKET", "test-bucket"),
    ("R2_ENDPOINT", "https://example.r2.cloudflarestorage.com"),
    ("R2_ACCESS_KEY_ID", "test-access-key"),
    ("R2_SECRET_ACCESS_KEY", "test-secret-key"),
    ("IAM_MANIFEST_URL", "https://iam.example.com/manifest.yaml"),
    (
        "REFERENCE_DATA_MANIFEST_URL",
        "https://reference-data.example.com/manifest.yaml",
    ),
    (
        "RELAY_MANIFEST_URL",
        "https://relay.example.com/manifest.yaml",
    ),
];

/// Puts environment variables back when dropped, even if the test panics.
struct Restore(Vec<(&'static str, Option<String>)>);

impl Drop for Restore {
    fn drop(&mut self) {
        for (name, value) in &self.0 {
            match value {
                Some(value) => std::env::set_var(name, value),
                None => std::env::remove_var(name),
            }
        }
    }
}

/// `config/production.yaml` parses and has every block the app needs.
///
/// Production config is only read on the server, so without this a missing
/// or misspelled key is first found as a crash after a deploy.
#[test]
#[serial]
fn production_config_is_complete() {
    let _restore = Restore(
        VARS.iter()
            .map(|(name, _)| (*name, std::env::var(name).ok()))
            .collect(),
    );
    for (name, value) in VARS {
        std::env::set_var(name, value);
    }

    let config = Config::from_folder(&Environment::Production, Path::new("config"))
        .expect("config/production.yaml should load");

    assert!(
        matches!(config.cache, CacheConfig::Redis(_)),
        "idempotency keys need a shared cache in production, got {:?}",
        config.cache
    );
    assert!(config.queue.is_some(), "workers need a queue in production");

    let settings: CoreSettings = config.settings().expect("`settings:` block");
    assert!(settings.events.enabled, "events are on in production");

    assert!(config.scheduler.is_some(), "scheduler block");
    assert_eq!(
        settings.storage.driver, "r2",
        "production stores files in R2"
    );
    let relay: RelaySettings = config
        .settings
        .as_ref()
        .and_then(|all| all.get("relay"))
        .map(|value| serde_json::from_value(value.clone()).expect("`relay` block"))
        .unwrap_or_default();
    assert!(relay.enabled);

    let cors = config
        .server
        .middlewares
        .cors
        .as_ref()
        .expect("`server.middlewares.cors` block");
    assert!(cors.enable, "CORS must be enabled");
    assert_eq!(cors.allow_origins, ["https://app.example.com"]);
}
