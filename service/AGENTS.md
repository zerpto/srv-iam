# Agent guide: Zerpto IAM service

## What this project is

`com.zerpto.srv.iam` is the **identity and access management (IAM) service**
of Zerpto. It owns users, tenants, roles, permissions, plan rules and sessions, issues
the shared JWT, and serves the `zerpto.iam.v1.Authorization/Authorize` gRPC
call that every other service makes on each request. It started as a copy of the
Zerpto microservice template (`com.zerpto.rstpl`); the template's building
blocks below are kept, the domain code is added here.

- Language: **Rust**. Framework: **[Loco](https://loco.rs)** ("Rails for Rust").
- Crate `zerpto_iam`, binary `zerpto_iam_cli`, service name `iam`
  (`SERVICE_NAME`, which names the NATS stream `IAM` and the subjects
  `zerpto.iam.>`), default database `zerpto_iam_development`.
- Source of truth for behavior: the monolith at
  **`/Users/fathur/Codes/com.zerpto.api-rs`**. Read it (read-only) to see how a
  feature currently works, then port it. Do not modify the monolith from here.
- The monolith is being split into **12 services**; this is the one that owns
  authentication and authorization. Which monolith modules (models,
  controllers, migrations, workers) belong here is not recorded. Ask the user
  before porting a module; the expected scope is users, tenants, roles and
  sessions.
- Keep anything that is not IAM out of this repo. Anything every service needs
  belongs in `zerpto-core`.

## Community edition

This repo is the **community edition** and holds **nothing proprietary**: no
billing client, no paid feature logic that needs a private service, and no
mention of a private repository as a dependency. A private enterprise service
depends on this crate as a library, so these are public extension points; keep
them stable:

- `plans::provider::PlanProvider`: who answers what a tenant's plan allows.
  `App::after_context` registers `plans::unlimited::Unlimited`. Code that needs
  the plan asks `plans::provider::provider(ctx)`, never a peer service, and
  never inside a database transaction. The `Authorize` gRPC server reads it
  the same way, and both decide through `plans::decide`.
- `app::initializers_with(grpc, subscriptions)`
- `extensions::core_manifests::CoreManifests`: the compiled-in core extensions,
  installed at boot. `App::after_context` inserts `CoreManifests::community()`.
- `grpc::server::routes(ctx)` (only this service's gRPC services; core adds
  health)
- `migration::Migrator` (re-exported from `lib.rs`)
- `http::v1::controllers::auth::signed_in`: starts a session for an extra login
  method.

README "Extending (enterprise)" shows how they are used.

## Extensions and permissions

- Permission keys are `{extension}.{resource}.{action}` and come only from
  extension manifests (`models::permissions::Catalogue`). A new IAM permission
  goes in `../federation/manifest.yaml`, never in Rust.
- Another service's manifest is never copied here: it lives in that project's
  `federation/` and `extensions::sync` fetches it.
- The configured address (`settings.extensions.core_manifest_urls`) is the core
  extension's `manifest_url`. A new core extension needs its entry in the three
  `config/*.yaml` and in `.env.example`.
- IAM's manifest is compiled in from `../federation/manifest.yaml`, without its
  app. Its `frontend.menu` holds the top-level menu entries, never Rust; only the
  groups of both menus are in `models/menu.rs`.
- Hold the lock `models::extensions::lock` in any transaction that changes the
  registry or what a tenant enabled.

## The shared crate: `zerpto-core`

Everything every service needs lives in a separate repo/crate,
**`zerpto-core`** (`https://github.com/zerpto/srv-core`), not in this service. A
service depends on it by git tag and updates by bumping the tag
(`[workspace.dependencies]` in `Cargo.toml`) and `Cargo.lock`; a change in a
local checkout is not picked up. One crate with feature flags: `events`,
`storage`, `relay` (HTTP, gRPC, settings and Sentry are always on). The
IAM enables all three; drop a flag it does not use.

- **In core:** the HTTP contract (`Problem`, envelope, paging, ids and the
  prefix registry, idempotency, filters, money, `Nullable`, `CurrentSession`),
  the IAM authorization client, the health service and proto,
  events (outbox, relay, consumer, cleanup task and the outbox migration),
  storage (backend, sniffing, uploads, signed links), relay, settings
  (`CoreSettings`), and the Sentry, SQL-report, events and gRPC server
  initializers.
- **In a service:** its `/v1` controllers, requests and responses, models and
  migrations, own protos, own gRPC services, event subscriptions, workers,
  tasks, config and `app.rs` wiring.
- **Never fix shared behavior inside a service.** Change it in core, release a
  tag, bump it here. Never copy core code into a service.
- Core pins the Loco version; this crate's `loco-rs` must be on the same minor.
  Upgrade both together.
- Core's own rules are in its `AGENTS.md`. Paths in "Building blocks" below
  without `http/v1`, `events/subscriptions.rs` or `grpc/server.rs` refer to
  `zerpto-core`'s `src/`.

## Hard rules

1. **`cargo loco ...` commands are allowed** (`generate`, `db migrate`,
   `db entities`, `task`, ...). **Every other `cargo` command is not**:
   `build`, `check`, `clippy`, `fmt`, `test`, `nextest`, `run`, in any scope
   and in the background. Do not run them; the user does when they want to.
2. **Never run tests.** Write and update tests, but leave running them to the
   user.
3. **Ask when unsure.** If a service boundary, data owner or API contract is
   unclear, ask the user. Do not guess.
4. **Never copy secrets.** The monolith's `.env` and `config/*.yaml` contain
   real keys and JWT secrets. Config reads everything from env vars
   (`get_env(name=..., default=...)`) with obvious placeholder defaults. Commit
   `.env.example` only, never `.env`.
5. **A question or a remark is not a task.** If the message asks something
   ("how", "why", "is it...?") or states an observation ("actually X is
   unused") and has no request to change something ("remove", "add", "fix",
   "update"), answer in words and do not edit any file. When a change follows
   from it, offer it in one line and wait for a yes. An approved plan or an
   explicit request counts as a go-ahead; a remark does not.

## Auth in a service

This service is the only one with a `users` table, login routes or mailers;
Loco's sample auth was removed and the real one is ported from the monolith.
Other services validate the shared
JWT with `zerpto_core::http::session::CurrentSession` (signature and expiry only) and reads
`user_id()` / `tenant_id()`. Authorization (roles, permissions, plans) is not in
the token: ask the IAM service over gRPC. `auth.jwt.secret` comes from
`JWT_SECRET` and must be identical in every service. Here the secret signs the
tokens this service issues, so changing it logs everyone out everywhere.

## Scaffold status

SQLite is not used. `loco-rs` is built with `default-features = false` and an
explicit feature list (no `db-sqlite`; **`db-postgres` must be in the list**,
or every connection fails with "Postgres support disabled"), and `sea-orm` only
enables `sqlx-postgres`. The lockfile resolves `loco-rs` 1.3, whose default
features differ from 1.2: re-check the feature list when upgrading Loco. Do not add SQLite back, not even for tests: tests run against
Postgres.

The base was generated once with `loco new` (Loco 1.2.0: Postgres, async
background workers, no assets; the sample `users`/auth code was then removed). The shared
building blocks were then written by hand and moved into `zerpto-core`; none of
it has been compiled yet, so the first `cargo check` may need small fixes. The
crate is now named `zerpto_iam`. That one-off `loco new` run is the
first `cargo`/`loco` execution that was authorized; rule 1 now allows
`cargo loco ...` and still forbids the rest.

**Generators first (standing rule).** If a Loco generator can create a file
(`cargo loco generate model|migration|scaffold|controller|worker|task|mailer|
data|...`), do not hand-write it: run the generator, then edit the generated
files. Hand-write only what no generator produces (request/response DTOs,
domain logic, gRPC code, config). Because UUIDv7 ids differ from Loco's
defaults, fix up after generating (`id` as `PkUuid`, the `before_save` id,
`Uuid` foreign keys).

## Architecture decisions

| Topic | Decision |
|---|---|
| Database | **One Postgres database per service.** The service owns its schema and migrations. No cross-service joins or foreign keys. Reference other services' data by ID (for example `tenant_id`, `user_id`) and treat those IDs as opaque. |
| Internal calls | **gRPC** between services (`tonic` + `prost`, `.proto` files as the contract). See "Building blocks" below. |
| Events | **NATS JetStream** with a transactional outbox. See "Building blocks" below. |
| Queue jobs | Loco `BackgroundQueue` on **Redis**. See "Building blocks" below. |
| External API | REST/JSON via Loco controllers under `/v1/...`, as in the monolith. Expose only what clients or the gateway need. |
| IAM | One JWT scheme shared across services (same claims, validated the same way). Do not re-implement user, session or tenant logic in each service. Confirm with the user which service owns auth. |
| Tenancy | Multi-tenant by `tenant_id`. Every query is scoped by tenant. |
| Config | All environment-specific values come from env vars. Keep `PORT`, `DATABASE_URL`, `GRPC_PORT` and peer service addresses configurable. |

Open items to settle with the user before they are baked in:
CI access to the `zerpto-core` and `zerpto-proto` repositories if they are
private, gateway or edge routing, how services discover each other (today: peer URLs
from env vars), the shared JWT secret handling, and authentication on gRPC
calls (none yet; add a tonic interceptor that checks the shared JWT).

## Building blocks already in this service

### `/v1` requests and responses (same contract as the monolith)
- Layout: `src/http/<version>/{controllers,requests,response}` in the service.
  Error type, envelope, paging and the `json` helper come from
  `zerpto_core::http`. A new
  version copies the `v1/` layout, is declared in `http/mod.rs` and mounted in
  `App::routes`.
- Success: `{data, meta}` via `http::json::json(status, data, meta)`;
  `EmptyMeta` when there is no meta. Lists use `PageListMeta` / `CursorListMeta`
  from `http/envelope.rs`.
- Errors: return `ApiResult<T>` and build a `Problem` (`bad_request`,
  `not_found`, `unprocessable`, ...) with a stable `code` (logged, not
  sent). The body is `{error: {message, request_id}}`, plus `details` on a
  validation failure (`ValidatedJson`, `Problem::validation`); no `code`, no
  `field`.
- Routes are declared in `http/v1/routing.rs`, one `Routes` per controller with
  `.layer(from_fn(with_request_id))`; handlers are `pub async fn` in
  `http/v1/controllers/<name>.rs` (see `health`). `App::routes` mounts them with
  `add_routes(http::v1::routing::routes())`.
- Request bodies live in `src/http/v1/requests/<area>.rs`: `#[derive(Validate)]`
  with rules on the fields (shared ones in `zerpto_core::validation::rules`, a
  struct-level `schema` rule for cross-field checks), taken in handlers as
  `ValidatedJson<T>`, then an `into_domain()` that trims and parses. Database
  rules (uniqueness, existence) go on the model. Response DTOs live in
  `src/http/v1/response/<area>.rs`. Both derive `ts-rs::TS`.

- Pagination: page-numbered lists take `Query<Paging>` (`?page=`, `?per_page=`,
  default 25, max 100) and respond with
  `PageListMeta { pagination: PagePagination::new(&page.meta, &uri) }`.
  Cursor lists take `Query<CursorPaging>` (`?limit=`, `?cursor=`, default 50,
  max 100) and respond with `CursorListMeta`. Both live in
  `http/paging.rs` and `http/envelope.rs`, as in the monolith.

### Error reporting (Sentry)
- `initializers/sentry.rs` starts Sentry when `SENTRY_DSN` is set; unset, it is
  a no-op. Panics are reported automatically, and `Problem`'s `internal` path
  reports every unexpected `/v1` error with a `request_id` tag.
- Expected errors (`bad_request`, `not_found`, ...) are not reported. Errors in
  workers, gRPC handlers and event consumers are only logged; call
  `sentry::capture_message` / `capture_error` there when one needs alerting.
- Never put secrets or personal data in error messages or tags.

### Shared HTTP helpers (`src/http/`)
Ported from the monolith so all services behave identically. Use them; do not
re-implement them per service.
- `idempotency.rs`: `Idempotency-Key` for create endpoints (`IdempotencyKey`:
  `from_headers`, `recall`, `remember`; 24 h, keyed by operation + caller, stores only a
  reference to the result). Needs the Redis cache in production.
- `filters.rs`: `QueryParam` parses `RFC 3339` timestamps and `YYYY-MM-DD`
  dates in query strings into `400` problems.
- `nullable.rs`: `Nullable<T>` for PATCH bodies (missing vs `null` vs value).
- `zerpto_core::helpers` (money): money and quantity are strings, never floats;
  `format_amount`, `format_price`, `format_quantity`, `parse_amount`, `IDR`.
- `initializers/query_log.rs`: in development only, adds the SQL a request ran
  to `meta.debug` and an `X-Query-Count` header.
- CORS is configured in `config/{development,production}.yaml`
  (`WEB_APP_URL`), with the `idempotency-key` header allowed.

### Permissions (gRPC to the IAM service, no cache)
- Roles change often and must apply at once, so nothing is cached. A handler
  calls `session.require_any(&ctx, &["accounting.expenses.read", "accounting.expenses.manage"]).await?`
  (`http/session.rs`), which asks the IAM service's
  `zerpto.iam.v1.Authorization/Authorize` for every request. Any-of
  semantics; `*` grants all.
- Failure modes: `403 permission_denied`, `403 plan_required`,
  `403 account_deactivated`, and **fail closed** `503 iam_unavailable`
  when IAM cannot answer.
- `proto/zerpto/iam/v1/authorization.proto` is owned by `zerpto-core`
  (one copy for the IAM server and every caller). Change it there, additive
  only, and release a tag. Config in a calling service: `IAM_GRPC_URL`.
  IAM is the IAM service, so it has no `IAM_GRPC_URL`: it answers
  from its own database.
- The IAM service implements the server side from `zerpto-proto`. Keep the hot path
  cheap there (indexed lookups); it is called on every request of every service.

### Storage (files)
- `ctx.storage` is built in `App::after_context` from `settings.storage`
  (`local` in development, Cloudflare R2 in production).
- `storage/sniff.rs`: decide a file's type from its bytes, never from the
  client's claim (`Sniffed::check_attachment`, `Sniffed::document`,
  `Sniffed::evidence`) and enforce size limits. `storage/files.rs`: `Files`
  (`store` as `<folder>/<tenant uuid>/<file uuid>.<ext>`, `read`, `remove`);
  `helpers::clean_filename`.
- `storage/uploads.rs`: `MultipartFiles` reads multipart files and
  `FileError::problem()` maps refusals to `422 unsupported_file_type` /
  `file_too_large`. `storage/file_links.rs`: `FileLinks`, signed,
  15-minute download links that need no sign-in, scoped by `purpose`; set
  `API_URL` in production so the links name the public address.
- Each service keeps its own file table (id UUIDv7, tenant, path, name, type,
  size); the bytes go through this module.

### Relay messages (queue job to the relay service)
- A service never sends email/WhatsApp itself. After its transaction commits it
  calls `RelayMessage { channel, recipient,
  template, tenant_id, data }.send(&ctx).await?`, which pushes a job onto the relay
  service's Redis queue (`RELAY_REDIS_URL`, queue `relay`,
  worker class `RelayWorker`).
- `relay/message.rs::RelayMessage` must match the relay service's
  worker args. Treat it as that service's contract and only add optional
  fields. Off unless `RELAY_ENABLED=true`.

### Scheduler
- `scheduler:` in `config/*.yaml` runs registered tasks on a cron schedule in a
  separate process: `cargo loco scheduler`. A task is a struct implementing
  `loco_rs::task::Task`, registered in `App::register_tasks`, runnable by hand
  with `cargo loco task <name> key:value`.
- This service ships one job, `outbox:cleanup` (daily 03:00, deletes outbox rows
  published more than 7 days ago). Add the service's own jobs beside it; delete
  the block if there are none. Run exactly one scheduler process per service,
  or jobs run twice.

### Release build
- `../.github/workflows/release-build.yaml` builds the Docker image when a
  GitHub release is published (tag `v1.2.3`), for amd64 and arm64, and publishes
  it to `ghcr.io/<owner>/iam`. It needs the repository secret `REPO_READ_TOKEN`.

### Queue jobs (Redis)
- Workers go in `src/workers/`, are registered in `App::connect_workers`, and
  are enqueued with `Worker::perform_later(&ctx, args)`. Args carry IDs, not
  models. Return `Err` to retry; never hand-roll retry loops.
- `config/{development,production}.yaml` use `BackgroundQueue` with
  `queue.kind: Redis` (`REDIS_URL`). Jobs only run in a worker process:
  `cargo loco start --server-and-worker` (or `--worker` separately).
  `config/test.yaml` stays `ForegroundBlocking`.

### gRPC
- Contracts are central: the `.proto` files live in the contract repo
  (`https://github.com/zerpto/contract-proto`) and services use the generated
  code through the `zerpto-proto` crate, one cargo feature per service package,
  pinned to a git tag (`[workspace.dependencies]` in `Cargo.toml`).
  **Never copy a `.proto` into this repo and never add a `build.rs`
  for one.** To add or change an rpc, change the proto in that repo (additive
  only, see its `AGENTS.md`), release a tag, then bump it here.
- Callers: a thin client in `src/grpc/<name>.rs` that builds the channel with
  `zerpto_core::helpers::channel(url)` (the URL comes from `settings`, never a
  literal) and maps the answer to a `Problem`. Example: `src/grpc/regions.rs`
  (reference-data, `REFERENCE_DATA_GRPC_URL`). It fails closed with `503`.
  Call peers before opening a database transaction, never inside one.
- Servers: implement the generated `*_server` trait in `src/grpc/<name>.rs` and
  register it in `src/grpc/server.rs`.
- The server is started by `zerpto-core`'s `GrpcServer` initializer, from
  `grpc::server::routes`, on `GRPC_PORT` (default 5001).
- `zerpto-core` still generates its own copy of the health and IAM
  contracts (package `zerpto.iam.v1`); the proto repo calls the IAM one
  `zerpto.iam.v1`. They must move together when core adopts `zerpto-proto`.

### Events (NATS JetStream)
- Subjects: `zerpto.<service>.<entity>.<event>`. Each service owns the stream
  named after it (`settings.service` upper-cased) covering `zerpto.<service>.>`.
- **Publish** by calling `outbox_events::Model::enqueue(&txn, subject, &event)`
  inside the same DB transaction as the change. Never publish to NATS directly
  from a handler. The relay (`events/relay.rs`) publishes outbox rows, and the
  event id doubles as `Nats-Msg-Id` for de-duplication.
- **Consume** by implementing `events::consumer::EventHandler` and adding a
  `Subscription` in `events/subscriptions.rs`. Delivery is at-least-once, so
  handlers must be idempotent (key on `event.id`). A durable name is
  `<this-service>-<purpose>`.
- The event body (`EventEnvelope.data`) is a contract: add fields, never change
  or remove them.
- Off by default (`EVENTS_ENABLED=false` in development). Local Postgres, Redis
  and NATS are run by the developer; there is no docker compose file here.

## Target layout of a service

Mirror the monolith so porting is mechanical:

```
src/
  app.rs            # Hooks impl: routes, workers, tasks, initializers
  bin/main.rs
  lib.rs
  http/             # this service's HTTP API (shared parts: zerpto-core)
    v1/ (v2/, ...)  # one directory per API version:
      controllers/  #   REST handlers: parse, call model, render
      requests/     #   request bodies with validate()
      response/     #   {data, meta} DTOs
  models/           # SeaORM entities (_entities) + domain logic (fat model)
  grpc/             # this service's tonic server (+ own protos)
  events/           # event subscriptions (outbox/relay: zerpto-core)
  workers/ tasks/ mailers/   # only if the service needs them
migration/          # this service's migrations only
proto/              # .proto contract for this service
config/             # development.yaml, test.yaml, production.yaml (env-driven)
tests/
```

Only include a directory when the service uses it.

## How to build out this service

1. Confirm with the user which monolith modules belong here (expected: users,
   tenants, roles, permissions, plans, sessions).
2. Port only the models, migrations, controllers and workers that belong to
   the service. Re-number or rewrite migrations so the service owns a clean,
   self-contained history.
3. Replace every cross-module dependency from the monolith (a model join, a
   direct function call) with either a stored ID plus a gRPC call, or data the
   caller passes in.
4. Define the gRPC contract in `proto/` first, then implement server and
   client.
5. Publish the service's domain events through the outbox and subscribe to the
   events it needs from peers.
6. Write tests next to the code. Do not run them.

## Coding conventions (inherited from the monolith)

### Models and primary keys
- Every table has a single primary key named **`id`**, a **UUIDv7**
  (`uuid::Uuid::now_v7()`). No auto-increment or serial integers, no composite
  keys, and no second public id column (no `pid`).
- Migration: `("id", ColType::PkUuid)`. There is no database default; the app
  generates the id. Foreign keys are `<entity>_id` columns of type `Uuid`.
- Entity: `#[sea_orm(primary_key, auto_increment = false)] pub id: Uuid`.
- Model: generate the id in `ActiveModelBehavior::before_save` when
  `insert && self.id.is_not_set()`, exactly as core's `events/outbox/model.rs` does
  (with its `entity.rs` and migration beside it). Copy that pattern for each new
  model.
- UUIDv7 sorts by creation time; use it for stable ordering and keyset paging
  (`id` as the cursor), with `created_at` as the tiebreaker where it matters.
- **API ids are prefixed**: `<prefix>-<uuid>` (e.g. `inv-0191a3c2-...`), same
  format as the monolith. The database, gRPC and events carry the plain UUID;
  only HTTP bodies, paths and queries use the prefixed form. Encode with
  `ApiIds::new(PREFIX).encode(&model.id)`, decode with `.parse(..)`, or
  `.require(..)` in a handler (wrong prefix or bad uuid -> `404 not_found`).
- Prefixes live in one registry, `zerpto-core`'s `src/http/ids.rs`; add yours
  there (a PR to core and a new tag). A prefix is registered once
  and never reused for another kind of record, across all 12 services: before
  adding one, check the monolith's `models/ids.rs` and every service's
  registry (the monolith already had to rename `trf` to `ctf`). Keep the shared
  ones (`usr`, `ten`) identical everywhere.

- **Fat model, slim controller.** Domain logic lives on the model. Handlers
  parse, call a model method and render.
- **Use the batteries.** Loco already provides the ORM, queue, scheduler,
  mailer, tasks, storage, cache and test harness. Do not add a crate for
  something Loco already does.
- **Generate, then edit** in the monolith's workflow (`cargo loco generate`).
  Since you cannot run cargo here, hand-write files and wire them completely:
  `mod` declarations, `app.rs` routes and the migrator list. A handler that
  exists but is not routed will 404.
- **`mod.rs` only declares and re-exports.** It holds `mod` / `pub mod`,
  `pub use` and a `//!` doc comment, nothing else. Give every struct, enum,
  trait, function, const and test its own file.
- DTOs derive TypeScript bindings with `ts-rs`, as in the monolith.
- Follow `.rustfmt.toml` and keep clippy clean (`-D warnings`).
- Use `zerpto-core` for pagination, filters, problem-details errors,
  idempotency keys and the session extractor instead of inventing new ones.

## Reference

- Monolith agent guide: `/Users/fathur/Codes/com.zerpto.api-rs/AGENTS.md`
- Loco skill shipped with this repo (matched to its `loco-rs` version):
  `.claude/skills/loco/`. Start with `SKILL.md`, read `doctrine.md` before
  writing code, and check `api-index.md` before guessing an API name. Recipes
  for models, endpoints, workers, tasks, mailers and tests are in `recipes/`.
- Loco docs: <https://loco.rs/docs/> and <https://loco.rs/AGENTS.md>

## Service dependencies in the README

The README has a `Service dependencies` section with two tables: the services that
depend on this service, and the services this service depends on. Whoever builds or
changes this service fills them and keeps them current in the same change: add, change
or remove a row whenever a call to or from another service (gRPC, event, queue job,
external API) appears, with an honest status (Wired, Stub, Not built). Replace the
`_TBD_` rows once the content is known; if it is not known, leave the placeholder
instead of guessing.

## Before you call a change done

Do not run these yourself (rule 1); mention one only when a change needs it:

```sh
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```
