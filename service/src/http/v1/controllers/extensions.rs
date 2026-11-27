use axum::http::{StatusCode, Uri};
use loco_rs::{controller::views::pagination::PagerMeta, prelude::*};
use serde::Deserialize;
use uuid::Uuid;
use zerpto_core::{
    helpers::json,
    http::{
        envelope::{EmptyMeta, PageListMeta, PagePagination},
        paging::Paging,
        problem::{ApiResult, Problem},
    },
    validation::ValidatedJson,
};

use crate::{
    extensions::{
        fetch::{self, FetchError},
        manifest::Manifest,
        manifest_check::{self, ManifestError},
    },
    http::{
        auth_session::AuthSession,
        // TODO: restore `PlatformOperator` with the `X-Platform-Key` check.
        // platform_key::{PlatformOperator, Viewer},
        platform_key::Viewer,
        v1::{
            requests::extensions::{RegisterExtensionRequest, RejectExtensionRequest},
            response::extensions::{Extension, ExtensionEnableMeta},
        },
    },
    models::{
        extensions::{self, ExtensionError},
        tenant_extensions,
    },
    settings::Settings,
};

const MANAGE: [&str; 1] = ["iam.extensions.manage"];

fn manifest_problem(err: &ManifestError) -> Problem {
    match err {
        ManifestError::Invalid(message) => {
            Problem::unprocessable("invalid_manifest", message.clone())
        }
        ManifestError::CoreApiUnsupported(message) => {
            Problem::unprocessable("core_api_unsupported", message.clone())
        }
        ManifestError::KeyCollision(message) => Problem::conflict("key_collision", message.clone()),
    }
}

fn extension_problem(err: &ExtensionError) -> Problem {
    match err {
        ExtensionError::NotFound => Problem::not_found("No such extension."),
        ExtensionError::Exists => Problem::conflict(
            "extension_exists",
            "This extension is already registered. Refresh it to pick up a new manifest.",
        ),
        ExtensionError::CoreKey => Problem::conflict(
            "core_extension",
            "This is a core extension, installed with the platform.",
        ),
        ExtensionError::IsCore => Problem::conflict(
            "extension_is_core",
            "A core extension is updated with the platform and is always enabled.",
        ),
        ExtensionError::KeyMismatch => Problem::unprocessable(
            "key_mismatch",
            "The manifest's `key` is not this extension's, and it cannot change.",
        ),
        ExtensionError::UnknownDependency(key) => Problem::unprocessable(
            "unknown_dependency",
            format!("The required extension `{key}` is not registered."),
        ),
        ExtensionError::DependencyNotSatisfied(entry) => Problem::conflict(
            "dependency_not_satisfied",
            format!("`{entry}` is not approved in a version that satisfies the range."),
        ),
        ExtensionError::NothingToReview => Problem::conflict(
            "nothing_to_review",
            "No manifest of this extension is waiting for review.",
        ),
        ExtensionError::Required(key) => Problem::conflict(
            "extension_required",
            format!("`{key}` depends on this extension. Disable it first."),
        ),
        // Reached when a waiting manifest is approved, where the contract
        // answers every refusal with a conflict.
        ExtensionError::Manifest(ManifestError::CoreApiUnsupported(message)) => {
            Problem::conflict("core_api_unsupported", message.clone())
        }
        ExtensionError::Manifest(err) => manifest_problem(err),
    }
}

/// Downloads and checks a manifest. Done before any database transaction.
async fn fetch_manifest(ctx: &AppContext, address: &str) -> Result<Manifest, Problem> {
    let allowed_hosts = Settings::load(ctx)?.extensions.allowed_hosts;
    let yaml = fetch::download(address, &allowed_hosts)
        .await
        .map_err(|err| match err {
            FetchError::UrlNotAllowed => Problem::unprocessable(
                "url_not_allowed",
                "The manifest address must use HTTPS or point to an allowed internal host.",
            )
            .field("manifest_url"),
            FetchError::Unreachable(cause) => Problem::unprocessable(
                "manifest_unreachable",
                format!("The manifest could not be fetched: {cause}."),
            )
            .field("manifest_url"),
        })?;
    let manifest = manifest_check::parse(&yaml).map_err(|err| manifest_problem(&err))?;
    // A core key is refused as such, whatever else its manifest says.
    if manifest_check::CORE_KEYS.contains(&manifest.key.as_str()) {
        return Err(extension_problem(&ExtensionError::CoreKey));
    }
    manifest_check::check(&manifest).map_err(|err| manifest_problem(&err))?;
    Ok(manifest)
}

async fn find(ctx: &AppContext, id: &str) -> Result<extensions::Model, Problem> {
    extensions::Model::find_by_api_id(&ctx.db, id)
        .await?
        .ok_or_else(|| extension_problem(&ExtensionError::NotFound))
}

fn operator_view(status: StatusCode, row: &extensions::Model) -> ApiResult {
    Ok(json(
        status,
        Extension::for_operator(row)?,
        EmptyMeta::default(),
    ))
}

/// The tenant of the session, checked to still be the user's.
async fn session_tenant(ctx: &AppContext, session: &AuthSession) -> Result<Option<Uuid>, Problem> {
    session.user(ctx).await?;
    Ok(session.scope(ctx).await?.map(|scope| scope.tenant.id))
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    status: Option<String>,
}

/// `GET /v1/iam/extensions`: everything for the operator, the approved ones
/// for a signed-in user.
///
/// # Errors
///
/// `400` for an unknown `status` or invalid paging. Fails too when the
/// extensions cannot be read.
#[debug_handler]
pub async fn list(
    State(ctx): State<AppContext>,
    viewer: Viewer,
    uri: Uri,
    Query(query): Query<ListQuery>,
    Query(paging): Query<Paging>,
) -> ApiResult {
    let pagination = paging.resolve()?;
    let status = query.status.as_deref();
    if status.is_some_and(|status| {
        ![
            extensions::PENDING_REVIEW,
            extensions::APPROVED,
            extensions::REJECTED,
        ]
        .contains(&status)
    }) {
        return Err(Problem::bad_request(
            "invalid_status",
            "`status` must be `pending_review`, `approved` or `rejected`.",
        )
        .field("status"));
    }

    let (rows, enabled) = match &viewer {
        Viewer::Operator => (extensions::Model::list(&ctx.db, status).await?, None),
        Viewer::User(session) => {
            let enabled = match session_tenant(&ctx, session).await? {
                Some(tenant_id) => {
                    tenant_extensions::Model::enabled_ids(&ctx.db, tenant_id).await?
                }
                None => std::collections::HashSet::new(),
            };
            let rows = if status.is_none_or(|status| status == extensions::APPROVED) {
                extensions::Model::list(&ctx.db, Some(extensions::APPROVED)).await?
            } else {
                Vec::new()
            };
            (rows, Some(enabled))
        }
    };

    let total_items = rows.len() as u64;
    let skip = usize::try_from((pagination.page - 1).saturating_mul(pagination.page_size))
        .unwrap_or(usize::MAX);
    let page = rows
        .iter()
        .skip(skip)
        .take(usize::try_from(pagination.page_size).unwrap_or(usize::MAX))
        .map(|row| {
            enabled.as_ref().map_or_else(
                || Extension::for_operator(row),
                |enabled| Extension::for_user(row, row.core || enabled.contains(&row.id)),
            )
        })
        .collect::<ModelResult<Vec<_>>>()?;
    let meta = PagerMeta {
        page: pagination.page,
        page_size: pagination.page_size,
        total_pages: total_items.div_ceil(pagination.page_size),
        total_items,
    };
    Ok(json(
        StatusCode::OK,
        page,
        PageListMeta {
            pagination: PagePagination::new(&meta, &uri),
        },
    ))
}

/// `POST /v1/iam/extensions`: register an extension from its manifest address.
///
/// # Errors
///
/// Refuses an address whose manifest cannot be fetched or is not accepted by
/// the registry. Fails too when the extension cannot be saved.
#[debug_handler]
pub async fn register(
    State(ctx): State<AppContext>,
    // TODO: X-Platform-Key check is off for now; anyone can call this.
    // _operator: PlatformOperator,
    ValidatedJson(req): ValidatedJson<RegisterExtensionRequest>,
) -> ApiResult {
    let address = req.manifest_url.trim();
    let manifest = fetch_manifest(&ctx, address).await?;
    let row = extensions::Model::register(&ctx.db, address, &manifest)
        .await?
        .map_err(|err| extension_problem(&err))?;
    operator_view(StatusCode::CREATED, &row)
}

/// `GET /v1/iam/extensions/{id}`
///
/// # Errors
///
/// `404` when there is no such extension, or a signed-in user asks for one that
/// is not approved. Fails too when it cannot be read.
#[debug_handler]
pub async fn show(
    State(ctx): State<AppContext>,
    viewer: Viewer,
    Path(id): Path<String>,
) -> ApiResult {
    match viewer {
        Viewer::Operator => operator_view(StatusCode::OK, &find(&ctx, &id).await?),
        Viewer::User(session) => {
            let tenant_id = session_tenant(&ctx, &session).await?;
            let row = find(&ctx, &id).await?;
            if !row.is_approved() {
                return Err(extension_problem(&ExtensionError::NotFound));
            }
            let enabled = row.core
                || match tenant_id {
                    Some(tenant_id) => tenant_extensions::Model::enabled_ids(&ctx.db, tenant_id)
                        .await?
                        .contains(&row.id),
                    None => false,
                };
            Ok(json(
                StatusCode::OK,
                Extension::for_user(&row, enabled)?,
                EmptyMeta::default(),
            ))
        }
    }
}

/// `POST /v1/iam/extensions/{id}/refresh`: fetch the manifest again.
///
/// # Errors
///
/// Refuses an unknown extension, a core one, and a manifest that cannot be
/// fetched or is not accepted. Fails too when it cannot be saved.
#[debug_handler]
pub async fn refresh(
    State(ctx): State<AppContext>,
    // TODO: X-Platform-Key check is off for now; anyone can call this.
    // _operator: PlatformOperator,
    Path(id): Path<String>,
) -> ApiResult {
    let current = find(&ctx, &id).await?;
    let Some(address) = current.manifest_url.as_deref().filter(|_| !current.core) else {
        return Err(extension_problem(&ExtensionError::IsCore));
    };
    let manifest = fetch_manifest(&ctx, address).await?;
    let row = extensions::Model::refresh(&ctx.db, current.id, &manifest)
        .await?
        .map_err(|err| extension_problem(&err))?;
    operator_view(StatusCode::OK, &row)
}

/// `POST /v1/iam/extensions/{id}/approve`
///
/// # Errors
///
/// Refuses an unknown extension and any change the registry does not allow.
/// Fails too when it cannot be saved.
#[debug_handler]
pub async fn approve(
    State(ctx): State<AppContext>,
    // TODO: X-Platform-Key check is off for now; anyone can call this.
    // _operator: PlatformOperator,
    Path(id): Path<String>,
) -> ApiResult {
    let current = find(&ctx, &id).await?;
    let row = extensions::Model::approve(&ctx.db, current.id)
        .await?
        .map_err(|err| extension_problem(&err))?;
    operator_view(StatusCode::OK, &row)
}

/// `POST /v1/iam/extensions/{id}/reject`
///
/// # Errors
///
/// Refuses an unknown extension and any change the registry does not allow.
/// Fails too when it cannot be saved.
#[debug_handler]
pub async fn reject(
    State(ctx): State<AppContext>,
    // TODO: X-Platform-Key check is off for now; anyone can call this.
    // _operator: PlatformOperator,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<RejectExtensionRequest>,
) -> ApiResult {
    let current = find(&ctx, &id).await?;
    let row = extensions::Model::reject(&ctx.db, current.id, req.reason.trim())
        .await?
        .map_err(|err| extension_problem(&err))?;
    operator_view(StatusCode::OK, &row)
}

/// `POST /v1/iam/extensions/{id}/enable`: switch it on for the company.
///
/// # Errors
///
/// `403` without the permission to manage extensions, and a refusal when the
/// registry does not allow enabling this one. Fails too when the change cannot
/// be saved.
#[debug_handler]
pub async fn enable(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let (row, also_enabled) = tenant_extensions::Model::enable(&ctx.db, access.tenant_id(), &id)
        .await?
        .map_err(|err| extension_problem(&err))?;
    Ok(json(
        StatusCode::OK,
        Extension::for_user(&row, true)?,
        ExtensionEnableMeta { also_enabled },
    ))
}

/// `POST /v1/iam/extensions/{id}/disable`: switch it off for the company.
///
/// # Errors
///
/// `403` without the permission to manage extensions, and a refusal when the
/// registry does not allow disabling this one. Fails too when the change cannot
/// be saved.
#[debug_handler]
pub async fn disable(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let row = tenant_extensions::Model::disable(&ctx.db, access.tenant_id(), &id)
        .await?
        .map_err(|err| extension_problem(&err))?;
    Ok(json(
        StatusCode::OK,
        Extension::for_user(&row, false)?,
        EmptyMeta::default(),
    ))
}
