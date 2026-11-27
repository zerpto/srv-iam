//! Installs the core extensions in the registry when the service starts, after the
//! migrations ran.
//!
//! The compiled-in manifests are installed at once; those with a configured address
//! are then fetched in the background.

use std::{collections::BTreeMap, time::Duration};

use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Initializer},
    Error, Result,
};
use sea_orm::DatabaseConnection;

use super::{core_manifests, core_manifests::CoreManifests, fetch};
use crate::{models::extensions, settings::Settings};

/// The wait after the first failed fetch. It doubles up to [`LAST_RETRY`].
const FIRST_RETRY: Duration = Duration::from_secs(2);
const LAST_RETRY: Duration = Duration::from_mins(1);

pub struct CoreExtensions;

#[async_trait]
impl Initializer for CoreExtensions {
    fn name(&self) -> String {
        "core-extensions".to_string()
    }

    async fn before_run(&self, ctx: &AppContext) -> Result<()> {
        let settings = Settings::load(ctx)?.extensions;
        let mut addresses = settings.core_manifest_urls;
        for address in addresses.values_mut() {
            *address = address.trim().to_string();
        }
        addresses.retain(|_, address| !address.is_empty());

        let bundled = ctx
            .shared_store
            .get::<CoreManifests>()
            .ok_or_else(|| Error::string("no core manifests are registered"))?
            .parse()?;
        let mut manifests = Vec::new();
        for manifest in bundled {
            // With an address, what was last fetched stays until the next fetch.
            if addresses.contains_key(&manifest.key)
                && extensions::Model::find_by_key(&ctx.db, &manifest.key)
                    .await?
                    .is_some()
            {
                continue;
            }
            manifests.push(manifest);
        }
        extensions::Model::install_core(&ctx.db, &manifests, &addresses).await?;

        if !addresses.is_empty() {
            tokio::spawn(fetch_all(ctx.db.clone(), addresses, settings.allowed_hosts));
        }
        Ok(())
    }
}

/// Fetches and installs each manifest, retrying the failed ones until none is
/// left.
async fn fetch_all(
    db: DatabaseConnection,
    mut waiting: BTreeMap<String, String>,
    allowed_hosts: String,
) {
    let mut delay = FIRST_RETRY;
    loop {
        let mut failed = BTreeMap::new();
        for (key, address) in waiting {
            match install(&db, &key, &address, &allowed_hosts).await {
                Ok(()) => tracing::info!(%key, %address, "core manifest installed"),
                Err(error) => {
                    tracing::warn!(%key, %address, %error, "core manifest not installed, will retry");
                    failed.insert(key, address);
                }
            }
        }
        if failed.is_empty() {
            return;
        }
        waiting = failed;
        tokio::time::sleep(delay).await;
        delay = (delay * 2).min(LAST_RETRY);
    }
}

async fn install(
    db: &DatabaseConnection,
    key: &str,
    address: &str,
    allowed_hosts: &str,
) -> std::result::Result<(), String> {
    let yaml = fetch::download(address, allowed_hosts)
        .await
        .map_err(|err| format!("{err:?}"))?;
    let manifest = core_manifests::accept(key, &yaml).map_err(|err| format!("{err:?}"))?;
    let addresses = BTreeMap::from([(key.to_string(), address.to_string())]);
    extensions::Model::install_core(db, &[manifest], &addresses)
        .await
        .map_err(|err| err.to_string())
}
