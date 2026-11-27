use async_trait::async_trait;
use loco_rs::{
    app::{AppContext, Hooks, Initializer},
    bgworker::{BackgroundWorker, Queue},
    boot::{create_app, BootResult, StartMode},
    config::Config,
    controller::AppRoutes,
    db::truncate_table,
    environment::Environment,
    task::Tasks,
    Result,
};
use migration::Migrator;
use std::{path::Path, sync::Arc};
use tonic::service::Routes;

use zerpto_core::{
    bootstrap,
    events::{cleanup::OutboxCleanup, consumer::Subscription, outbox},
    initializers::{Events, GrpcServer, NotFound, QueryLog, Sentry},
};

use crate::{
    events::subscriptions::subscriptions,
    extensions::{core_manifests::CoreManifests, sync::CoreExtensions},
    grpc, http,
    plans::{provider::PlanProvider, unlimited::Unlimited},
    workers::downloader::DownloadWorker,
};

/// The initializers of `Hooks::initializers`, with the gRPC services and event
/// subscriptions to start. A service that extends this one passes functions
/// that return these ones plus its own.
#[must_use]
pub fn initializers_with(
    grpc: fn(&AppContext) -> Result<Routes>,
    subscriptions: fn() -> Vec<Subscription>,
) -> Vec<Box<dyn Initializer>> {
    vec![
        Box::new(Sentry),
        Box::new(NotFound),
        Box::new(QueryLog),
        Box::new(CoreExtensions),
        Box::new(GrpcServer(grpc)),
        Box::new(Events(subscriptions)),
    ]
}

pub struct App;
#[async_trait]
impl Hooks for App {
    fn app_name() -> &'static str {
        env!("CARGO_CRATE_NAME")
    }

    fn app_version() -> String {
        format!(
            "{} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("BUILD_SHA")
                .or(option_env!("GITHUB_SHA"))
                .unwrap_or("dev")
        )
    }

    async fn boot(
        mode: StartMode,
        environment: &Environment,
        config: Config,
    ) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, environment, config).await
    }

    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        let ctx = bootstrap::after_context(ctx).await?;
        // A service that extends this one inserts its own provider over it.
        ctx.shared_store
            .insert::<Arc<dyn PlanProvider>>(Arc::new(Unlimited));
        // Likewise, it may insert a list with more core extensions.
        ctx.shared_store.insert(CoreManifests::community());
        Ok(ctx)
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(initializers_with(grpc::server::routes, subscriptions))
    }

    fn routes(_ctx: &AppContext) -> AppRoutes {
        AppRoutes::with_default_routes() // controller routes below
            .add_routes(http::v1::routing::routes())
    }
    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        queue.register(DownloadWorker::build(ctx)).await?;
        Ok(())
    }

    #[allow(unused_variables)]
    fn register_tasks(tasks: &mut Tasks) {
        // tasks-inject (do not remove)
        tasks.register(OutboxCleanup);
    }
    async fn truncate(ctx: &AppContext) -> Result<()> {
        truncate_table(&ctx.db, outbox::Entity).await?;
        Ok(())
    }
    async fn seed(_ctx: &AppContext, _base: &Path) -> Result<()> {
        Ok(())
    }
}
