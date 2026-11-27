# Zerpto IAM service

Identity and access management for Zerpto: users, tenants, roles, permissions,
plans and sessions, plus the shared JWT and the gRPC `Authorization` service
every other service calls. Rust on [Loco](https://loco.rs), with everything
shared across services in the `zerpto-core` crate. Agents and contributors: read
[AGENTS.md](AGENTS.md) first.

## Ports

| What | Port | Setting |
|---|---|---|
| HTTP (REST `/v1`, health probes) | `4001` | `PORT` |
| gRPC (service to service) | `5001` | `GRPC_PORT` |

Every Zerpto service has its own pair, so all of them run side by side: Kong
gateway `4000`, then HTTP `400N` and gRPC `500N` for service number N (see the
table in the monorepo README). Peer services are reached through the
`*_GRPC_URL` variables, which default to the peer's gRPC port.

## Run it

Postgres and Redis (and NATS when `EVENTS_ENABLED=true`) must already be running
locally; point `DATABASE_URL`, `REDIS_URL` and `NATS_URL` at them.

Cargo downloads `zerpto-core` and `zerpto-proto` from GitHub at the tags in [`Cargo.toml`](Cargo.toml).
If the repositories are private, set `CARGO_NET_GIT_FETCH_WITH_CLI=true` so Cargo uses your Git
credentials.

```sh
cargo loco start --server-and-worker       # HTTP on :4001, gRPC on :5001
curl localhost:4001/v1/health              # {"data":{"status":"ok"},"meta":{}}
```

Needs `protoc` (`brew install protobuf`). Config is in `config/*.yaml` and every
value comes from an env var, so nothing is hard-coded per machine.

| Variable | Used for |
|---|---|
| `DATABASE_URL` | This service's own Postgres database |
| `JWT_SECRET` | Token check; identical in every service |
| `REDIS_URL` | Queue jobs and (production) the idempotency cache |
| `EVENTS_ENABLED`, `NATS_URL` | Event streaming (off by default in development) |
| `SENTRY_DSN` | Error reporting (off when empty) |
| `WEB_APP_URL` | CORS origin of the web app |
| `REFERENCE_DATA_GRPC_URL` | Reference-data service (gRPC), asked to check a tenant's address |
| `PLATFORM_API_KEY` | The platform operator's key (`X-Platform-Key`) for the extension registry. Empty refuses every operator call. **Temporarily not checked** on register, refresh, approve and reject, which are open to anyone who can reach the service (see `controllers/extensions.rs`) |
| `EXTENSION_ALLOWED_HOSTS` | Internal hosts a manifest may be fetched from over plain HTTP, comma-separated. HTTPS needs no entry |
| `IAM_MANIFEST_URL`, `REFERENCE_DATA_MANIFEST_URL`, `RELAY_MANIFEST_URL` | Where those core extensions serve `manifest.yaml`, fetched at startup and stored as `manifest_url`. Required in production; empty means not fetched |
| `BILLING_MANIFEST_URL` | The same for billing. Optional: empty leaves billing out |

## Extensions and permissions

Every permission key is `{extension}.{resource}.{action}` and comes from an extension's `manifest.yaml`. The core
extensions (`iam`, `reference-data`, `relay`) are installed with the platform and always enabled. The platform
operator registers and reviews others under `/v1/iam/extensions`; each tenant enables the approved ones. A new tenant
starts with all approved extensions. Permissions, `/v1/auth/me` and `/v1/iam/menu` only count core and enabled
extensions.

Each core extension keeps its `manifest.yaml` in its own project's `federation/`, which serves it. IAM fetches the
configured addresses when it starts, in the background and with retries, installs what it gets without review, and keeps
the stored version while an address does not answer. Restart IAM after a manifest changes.

- `iam`: [`../federation/manifest.yaml`](../federation/manifest.yaml). It is also compiled in, without its app, so its
  permissions exist before the first fetch. Without `IAM_MANIFEST_URL` that copy is installed, and the web app shows its
  built-in settings pages.
- `reference-data`: listed only once it has been fetched.
- `relay`: listed only once it has been fetched.
- `billing`: listed only once it has been fetched, and only when `BILLING_MANIFEST_URL` is set.

The configured address is stored as the extension's `manifest_url`, also before the first successful fetch for an
extension already installed. The platform operator sees it; signed-in users get `null`. Core extensions cannot be
refreshed through the API.

A manifest adds sidebar entries (`frontend.menu`) and tenant settings entries (`frontend.settings_menu`), and
`GET /v1/iam/menu` returns both. Only a core extension may leave out an
entry's `group` (a top-level entry) or `permission` (every signed-in user sees it); IAM's own manifest declares
Dashboard, Approvals and Reports that way. A core extension may declare entries without an app (no `mode`, `remote`, `expose` or
`shared`).

IAM's own screens are in [`../federation`](../federation).

## Service dependencies

The gRPC contracts live in `contract.proto`; this service reaches its peers
through the `*_GRPC_URL` variables.

### Services that depend on this service

Who calls or listens to IAM, and what they need from it.

| Service | Communication | What it needs | Status now |
| --- | --- | --- | --- |
| Every other service | gRPC (`zerpto.iam.v1.Authorization/Authorize`) | Roles, permissions and plan check on every request (`require_any`) | Built: `src/grpc/authorization.rs` |
| Billing (used by the enterprise edition) | gRPC (`zerpto.iam.v1.Seats/GetSeatsInUse`) | Seats in use of a tenant: active members plus pending invitations (an unknown tenant is 0) | Built: `src/grpc/seats.rs` |
| Billing (used by the enterprise edition) | gRPC (`zerpto.iam.v1.PhoneVerifications/GetPhoneVerification`) | Whether a user has a verified phone and the stored phone hash, never the number (an unknown user is unverified with an empty hash), for one free trial per person and phone | Built: `src/grpc/phone_verifications.rs` |
| Accounting | Streaming event (NATS JetStream, outbox) | A tenant-created event, to seed the new tenant's chart of accounts | Not done: IAM does not publish it yet, and Accounting does not subscribe |

These servers are registered in `src/grpc/server.rs`, answer from this service's database and answer
`invalid_argument` for a malformed id. `Seats` and `PhoneVerifications` take plain UUIDs. `Authorize` takes API ids
(`usr-...`, `ten-...`), as its contract says: callers forward what the access token carries. It decides by the same
rules as IAM's own endpoints (`src/plans/decide.rs`); a non-owner writing on a locked plan is answered `DENIED`,
because the contract has no read-only decision.

### Services this service depends on

What IAM needs from other services. Rows marked "Wired" call the peer and fail
closed with `503` when it cannot answer.

| Affected endpoint | Service needed | Communication | Reason | Status now |
| --- | --- | --- | --- | --- |
| `POST /v1/auth/sign-up` | Relay | Queue job (Redis) | Email the confirmation link (`auth.verify_email`) | Queue job; off unless `RELAY_ENABLED=true`, the payload is logged |
| `POST /v1/auth/reset-password` | Relay | Queue job (Redis) | Email the reset link (`auth.reset_password`) | Same |
| `POST /v1/auth/phone/code` | Relay | Queue job (Redis) | Send the WhatsApp or SMS code (`auth.phone_code`) | Same |
| `POST /v1/iam/users` | Relay | Queue job (Redis) | Email the invitation (`auth.invitation`) | Same |
| `POST /v1/iam/extensions`, `POST /v1/iam/extensions/{id}/refresh` | The extension being registered | HTTP `GET` of its `manifest.yaml` | Read what the extension declares (`url_not_allowed`, `manifest_unreachable`, `invalid_manifest`) | Built: `src/extensions/fetch.rs`. No service serves a manifest yet |
| `POST /v1/iam/tenants`, `PATCH /v1/iam/tenant` | ReferenceData | gRPC (`zerpto.referencedata.v1` `Regions.ResolveAddress`, via `zerpto-proto`) | Check the address `region_id` and fill province, district and so on (`unknown_region`, `region_required`, `region_too_shallow`, `country_required`) | Wired. Fails closed: `503 reference_data_unavailable` when the address cannot be checked. Waiting for the reference-data server |

Plans are not a peer here: every permission check asks the registered `PlanProvider`, which in this community edition
locks nothing (see "Extending (enterprise)"). `/v1/auth/me` therefore reports `access: "full"` and
`trial_available: false`.

## The HTTP layer

Every endpoint lives under `src/http/<version>/` in three folders, and the
pieces they use come from `zerpto_core::http`:

```
src/http/v1/
  requests/     what the client sends, with validate()
  response/     what the service returns (DTOs)
  controllers/  handlers: parse, call a model method, render
  routing.rs    which handler answers which path
```

Adding an endpoint, for example `POST /v1/widgets` (names are illustrative):

**1. Response** (`response/widgets.rs`). DTOs derive `TS` for TypeScript
bindings, and ids leave the service in their prefixed form.

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zerpto_core::http::ids::ApiIds;

use crate::models::widgets;

#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Widget {
    pub id: String,
    pub name: String,
}

impl From<&widgets::Model> for Widget {
    fn from(widget: &widgets::Model) -> Self {
        Self { id: ApiIds::new(widgets::PREFIX).encode(&widget.id), name: widget.name.clone() }
    }
}
```

**2. Request** (`requests/widgets.rs`). Rules go on the fields with the
`validator` crate, and `ValidatedJson` checks them before the handler runs.
Shared rules (`name`, `code`, `amount`, `tax_id`, `date`) are in
`zerpto_core::validation::rules`; a rule that spans fields is one struct-level
`schema` rule (for example `rules::date_range`). `into_domain()` then trims and
parses the checked body into the value the model takes.

```rust
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;

#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct CreateWidgetRequest {
    #[validate(custom(function = "zerpto_core::validation::rules::name"))]
    pub name: String,
}

impl CreateWidgetRequest {
    #[must_use]
    pub fn into_domain(self) -> String {
        self.name.trim().to_string()
    }
}
```

A failed rule never reaches the handler. The client gets a `422` with every
problem listed (see "What the client sees"). Rules that need the database, such
as uniqueness, belong on the model and return a `409`.

**3. Controller** (`controllers/widgets.rs`). Keep it thin: check who is calling,
take the checked body, call the model, render with `json(status, data, meta)`.

```rust
use axum::{extract::State, http::StatusCode};
use loco_rs::prelude::*;
use zerpto_core::{
    helpers::json,
    http::{
        envelope::EmptyMeta, ids::{ApiIds, TENANT}, problem::ApiResult,
        session::CurrentSession,
    },
    validation::ValidatedJson,
};

use crate::{http::v1::{requests::widgets::CreateWidgetRequest, response::widgets::Widget}, models::widgets};

#[debug_handler]
pub async fn create(
    session: CurrentSession,
    State(ctx): State<AppContext>,
    ValidatedJson(body): ValidatedJson<CreateWidgetRequest>,
) -> ApiResult {
    // Asks the IAM service, every time; 403 if not allowed.
    session.require_any(&ctx, &["widgets.records.manage"]).await?;
    let tenant_id = ApiIds::new(TENANT).require(session.require_tenant()?)?;

    let widget = widgets::Model::create(&ctx.db, tenant_id, body.into_domain()).await?;
    Ok(json(StatusCode::CREATED, Widget::from(&widget), EmptyMeta::default()))
}
```

**4. Route** (`routing.rs`). A route group has one prefix, so add one `Routes`
per controller to the returned list. Keep the `with_request_id` layer so errors
quote the request id.

```rust
Routes::new()
    .prefix("/v1/widgets")
    .add("/", post(widgets::create))
    .layer(from_fn(with_request_id)),
```

Declare each new file in its folder's `mod.rs`; a handler that is not routed
returns 404.

### What the client sees

Success is always `{ "data": ..., "meta": {...} }`. Errors are always
`{ "error": { "message", "request_id" } }`, plus `details` (a list of
`{field, message}`) on a failed validation (`422`); there is no `code` or
`field`, so clients go by the HTTP status. Build errors with `Problem::bad_request`, `unauthorized`, `forbidden`,
`not_found`, `conflict` or `unprocessable`. Any other error (a database failure,
for instance) becomes a generic `500 internal_error`, is logged and goes to
Sentry with the request id.

### Lists and paging

```rust
Query(paging): Query<Paging>,     // ?page=2&per_page=25 (default 25, max 100)
uri: Uri,
// ...
let page = widgets::Model::list(&ctx.db, tenant_id, &paging.resolve()?).await?;
Ok(json(
    StatusCode::OK,
    page.page.iter().map(Widget::from).collect::<Vec<_>>(),
    PageListMeta { pagination: PagePagination::new(&page.meta, &uri) },
))
```

For long or fast-changing lists use `CursorPaging` (`?limit=&cursor=`) and
`CursorListMeta` instead. `meta.pagination` carries `self`, `first`, `prev`,
`next` and `last` links.

### Ids

Inside the service, an id is a plain UUIDv7. Over HTTP it is prefixed:
`wdg-0191a3c2-...`. Encode with `ApiIds::new(PREFIX).encode(&id)`, decode with
`ApiIds::new(PREFIX).require(text)?` (a wrong prefix or a malformed id is a `404`).
Register a new prefix in `zerpto-core`'s `http/ids.rs`; prefixes are unique across
all services.

### Other helpers

- **Create endpoints that must not run twice:** `idempotency::IdempotencyKey`
  (`from_headers`, `recall`, `remember`) (the `Idempotency-Key` header, kept 24 hours).
- **PATCH bodies:** `Nullable<T>` tells a missing field from `null` from a value.
- **Query-string dates and timestamps:** `filters::QueryParam::new(value, name, code).timestamp()` / `.date()`.
- **Money and quantity:** strings, never floats; see `helpers::{format_amount,
  format_price, format_quantity, parse_amount}`.
- **Uploads and downloads:** `storage::uploads::MultipartFiles`, `storage::sniff::Sniffed`,
  `storage::file_links::FileLinks` (feature `storage`).

### Testing an endpoint

Request tests boot the app against the test Postgres (`config/test.yaml`, which
recreates the database on each run). See `tests/requests/health.rs`:

```rust
request::<App, _, _>(|request, _ctx| async move {
    let response = request.get("/v1/health").await;
    assert_eq!(response.status_code(), 200);
}).await;
```

Endpoints that need a signed-in caller need a token signed with the test
`JWT_SECRET`, and a stand-in for the IAM service, since permission checks
call it over gRPC.

## Extending (enterprise)

This repository is the **community edition**. It holds nothing proprietary: no billing, unlimited seats, nothing locked,
no trial. An enterprise edition lives in its **own private repository**, depends on this crate as a library and adds to
it. Dependencies point one way: nothing here names the enterprise repository.

```
srv.iam (community, this repo)        srv.iam-enterprise (private)
  zerpto_iam lib + `iam` binary  <---   Cargo.toml: zerpto_iam (path now, git tag later)
  controllers, models, routes           src/app.rs: its own Hooks, calling community's
  plans::PlanProvider + Unlimited       src/plans/billing.rs: its own PlanProvider
  migrations                            migration/: community's list plus its own
                                        config/, .cargo/config.toml, src/bin/main.rs
```

| To | Use |
|---|---|
| Decide what a tenant's plan allows | `plans::provider::PlanProvider`, inserted in `after_context` |
| Add core extensions and their permissions | `extensions::core_manifests::CoreManifests::community().with(..)`, inserted in `after_context` |
| Add REST endpoints | `AppRoutes::add_routes` after community's `routes` |
| Guard them | `http::auth_session::AuthSession::authorize` |
| Add a login method (SSO) | `http::v1::controllers::auth::signed_in` |
| Add gRPC services | `grpc::server::routes(ctx)?.add_service(..)` |
| Consume more events, add initializers | `app::initializers_with(grpc, subscriptions)` |
| Add tables | `migration::Migrator::migrations()` plus its own |

### How it runs

A library alone cannot run `cargo loco`: the alias is `cargo run --`, which runs the binary of the folder you are in.
Community runs here, as above. Enterprise runs in its own folder with its own binary, which serves the community
endpoints and its own in one process, on one database. It does not run or proxy to the `iam` binary.

### Example

A sketch of the enterprise side. **It is never compiled**, so check it against the code.

`Cargo.toml`

```toml
[workspace]

[package]
name = "iam_enterprise"
version = "0.1.0"
edition = "2021"
default-run = "iam-enterprise"

[dependencies]
zerpto_iam = { path = "../srv.iam/service" } # git + tag later
# The same source and tag as community.
zerpto-core = { git = "https://github.com/zerpto/srv-core", tag = "v0.1.0", features = ["events", "storage", "relay"] }
# The same version and features as community; `generators` keeps `cargo loco generate` working.
loco-rs = { version = "1.3", default-features = false, features = ["auth", "cli", "generators", "with-db", "db-postgres", "cache_inmem", "cache_redis", "worker_redis"] }
migration = { path = "migration" }
# Plus what the code below uses: tokio, async-trait, axum, dotenvy, uuid, tonic, ...

[[bin]]
name = "iam-enterprise"
path = "src/bin/main.rs"
```

`.cargo/config.toml` (a dependency's alias does not carry over)

```toml
[alias]
loco = "run --"
```

`src/bin/main.rs`

```rust
use iam_enterprise::app::App;
use loco_rs::cli;
use migration::Migrator;

#[tokio::main]
async fn main() -> loco_rs::Result<()> {
    dotenvy::dotenv().ok();
    cli::main::<App, Migrator>().await
}
```

`src/app.rs`: community stays whole, enterprise appends

```rust
use zerpto_iam::{
    app::{initializers_with, App as Community},
    plans::provider::PlanProvider,
};

pub struct App;

#[async_trait]
impl Hooks for App {
    // app_name, app_version: enterprise's own

    async fn boot(mode: StartMode, env: &Environment, config: Config) -> Result<BootResult> {
        create_app::<Self, Migrator>(mode, env, config).await
    }

    async fn after_context(ctx: AppContext) -> Result<AppContext> {
        let ctx = Community::after_context(ctx).await?;
        // Replaces community's `Unlimited`.
        ctx.shared_store
            .insert::<Arc<dyn PlanProvider>>(Arc::new(BillingPlans::default()));
        Ok(ctx)
    }

    async fn initializers(_ctx: &AppContext) -> Result<Vec<Box<dyn Initializer>>> {
        Ok(initializers_with(grpc_routes, subscriptions))
    }

    fn routes(ctx: &AppContext) -> AppRoutes {
        Community::routes(ctx) // every community endpoint
            .add_routes(http::v1::routing::routes()) // plus enterprise ones
    }

    async fn connect_workers(ctx: &AppContext, queue: &Queue) -> Result<()> {
        Community::connect_workers(ctx, queue).await?;
        queue.register(AuditExportWorker::build(ctx)).await?;
        Ok(())
    }

    fn register_tasks(tasks: &mut Tasks) {
        Community::register_tasks(tasks);
        tasks.register(AuditCleanup);
    }

    async fn truncate(ctx: &AppContext) -> Result<()> {
        Community::truncate(ctx).await?;
        truncate_table(&ctx.db, audit_logs::Entity).await?;
        Ok(())
    }

    async fn seed(ctx: &AppContext, base: &Path) -> Result<()> {
        Community::seed(ctx, base).await
    }
}
```

`src/grpc/server.rs` and `src/events/subscriptions.rs`

```rust
use zerpto_core::tonic::service::Routes;

/// Community's gRPC services plus enterprise's. Health is added by core.
pub fn grpc_routes(ctx: &AppContext) -> Result<Routes> {
    Ok(zerpto_iam::grpc::server::routes(ctx)?
        .add_service(AuditServer::new(AuditService { db: ctx.db.clone() })))
}

/// Community's subscriptions plus enterprise's, with their own durable names.
pub fn subscriptions() -> Vec<Subscription> {
    let mut all = zerpto_iam::events::subscriptions::subscriptions();
    all.push(invoice_paid());
    all
}
```

`src/plans/billing.rs`

```rust
#[async_trait]
impl PlanProvider for BillingPlans {
    async fn plan(&self, ctx: &AppContext, tenant_id: Uuid) -> Result<Plan, Problem> {
        // ask billing over gRPC; fail closed when it cannot answer
    }
    // features, trial_available: likewise
}
```

The billing client community used to have (`src/grpc/billing.rs`, in git history) is the starting point.

`src/http/v1/routing.rs`: a community prefix can be reused with new paths

```rust
pub fn routes() -> Vec<Routes> {
    vec![
        Routes::new()
            .prefix("/v1/auth") // next to community's /v1/auth/sign-in
            .add("/sso", post(sso::sign_in))
            .layer(from_fn(with_request_id)),
        Routes::new()
            .prefix("/v1/iam/audit-logs")
            .add("/", get(audit_logs::list))
            .layer(from_fn(with_request_id)),
    ]
}
```

`src/http/v1/controllers/sso.rs`: reuses community's model and session logic

```rust
use zerpto_iam::{http::v1::controllers::auth::signed_in, models::users};

#[debug_handler]
pub async fn sign_in(
    State(ctx): State<AppContext>,
    ValidatedJson(req): ValidatedJson<SsoSignInRequest>,
) -> ApiResult {
    let email = identity_provider::verify(&ctx, &req.assertion).await?; // enterprise
    let user = users::Model::find_by_email(&ctx.db, &email) // community
        .await?
        .ok_or_else(|| Problem::unauthorized("unknown_user", "No account for this sign-in."))?;
    signed_in(&ctx, &user).await // community
}
```

`src/http/v1/controllers/audit_logs.rs`: community auth guards an enterprise endpoint

```rust
#[debug_handler]
pub async fn list(State(ctx): State<AppContext>, session: AuthSession) -> ApiResult {
    let access = session.authorize(&ctx, &["iam.users.manage"]).await?;
    let logs = audit_logs::Model::list(&ctx.db, access.tenant_id()).await?;
    Ok(json(StatusCode::OK, AuditLog::list(logs), EmptyMeta::default()))
}
```

`migration/src/lib.rs`

```rust
pub use sea_orm_migration::prelude::*;

mod m20261201_000001_audit_logs;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        let own: Vec<Box<dyn MigrationTrait>> = vec![
            Box::new(m20261201_000001_audit_logs::Migration),
            // inject-above (do not remove this comment)
        ];
        let mut all = zerpto_iam::migration::Migrator::migrations();
        all.extend(own);
        all
    }
}
```

Keep own migrations in a `vec![]` ending with the `inject-above` comment, where `cargo loco generate` adds new ones.

### Rules

- Add **new** method and path pairs and new gRPC services only: a duplicate panics at startup.
- Own tables only. Never alter a community table.
- Migration names and event durable names must be unique across both repositories.
- Use the same `zerpto-core` and `tonic` source and version as community.
- Keep its own `config/*.yaml`, containing every community `settings` key, and its own `.env`.
- A database that ran enterprise migrations cannot go back to the community binary: it refuses applied migrations it
  does not know.

### Commands in the enterprise folder

- `cargo loco generate` writes only into the enterprise project.
- `task`, `scheduler` and `db migrate` include community's only if the hooks and `Migrator` call community's.
- `db entities` reads the whole database. Check its output for community tables and delete those duplicates.

## Open questions

Not designed yet:

1. Should enterprise run code inside community flows (audit log, SSO enforcement)?
2. Should it be able to replace a community endpoint or change its response?
3. Do community installs have reference-data (tenant address check) and relay (email, WhatsApp), and are
   `zerpto-core` and `contract.proto` public?

## More

- [AGENTS.md](AGENTS.md): architecture, rules and every building block (events,
  queue jobs, scheduler, storage, relay, Sentry).
- [Loco documentation](https://loco.rs/docs/)
