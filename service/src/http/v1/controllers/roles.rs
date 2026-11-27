use axum::http::{StatusCode, Uri};
use loco_rs::prelude::*;
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
    http::{
        auth_session::AuthSession,
        v1::{
            requests::roles::{CreateRoleRequest, UpdateRoleRequest},
            response::roles::Role,
        },
    },
    models::roles::{self, RoleError},
};

const READ: [&str; 2] = ["iam.roles.read", "iam.roles.manage"];
const MANAGE: [&str; 1] = ["iam.roles.manage"];

fn role_problem(err: &RoleError) -> Problem {
    match err {
        RoleError::NotFound => Problem::not_found("No such role."),
        RoleError::NameTaken => {
            Problem::conflict("role_name_taken", "Another role already has this name.")
                .field("name")
        }
        RoleError::UnknownPermission(key) => Problem::unprocessable(
            "unknown_permission",
            format!("`{key}` is not a permission. See GET /v1/iam/permissions."),
        )
        .field("permissions"),
        RoleError::CannotGrant(key) => Problem::forbidden(
            "cannot_grant",
            format!("You cannot give `{key}` because you do not hold it yourself."),
        ),
        RoleError::IsSystem => Problem::conflict(
            "role_is_system",
            "Built-in roles cannot be changed or deleted. Clone it to make your own.",
        ),
        RoleError::InUse => Problem::conflict(
            "role_in_use",
            "Someone still holds this role. Move them to another role first.",
        ),
        RoleError::PermissionsRequired => Problem::unprocessable(
            "permissions_required",
            "Send `permissions`, or `clone_from_role_id` to copy them from a role.",
        )
        .field("permissions"),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    #[serde(rename = "type")]
    role_type: Option<String>,
}

async fn find(ctx: &AppContext, tenant_id: Uuid, id: &str) -> Result<roles::Model, Problem> {
    roles::Model::find_visible(&ctx.db, tenant_id, id)
        .await?
        .ok_or_else(|| role_problem(&RoleError::NotFound))
}

/// `GET /v1/iam/roles`: built-in roles and the company's own.
///
/// # Errors
///
/// `400` for an unknown `type` or invalid paging, and `403` without the
/// permission to read roles.
#[debug_handler]
pub async fn list(
    State(ctx): State<AppContext>,
    session: AuthSession,
    uri: Uri,
    Query(query): Query<ListQuery>,
    Query(paging): Query<Paging>,
) -> ApiResult {
    let access = session.authorize(&ctx, &READ).await?;
    let pagination = paging.resolve()?;
    if let Some(role_type) = &query.role_type {
        if ![roles::SYSTEM, roles::CUSTOM].contains(&role_type.as_str()) {
            return Err(Problem::bad_request(
                "invalid_type",
                "`type` must be `system` or `custom`.",
            )
            .field("type"));
        }
    }
    let (page, total_items) = roles::Model::list_visible(
        &ctx.db,
        access.tenant_id(),
        query.role_type.as_deref(),
        &pagination,
    )
    .await?;
    let meta = PagerMeta {
        page: pagination.page,
        page_size: pagination.page_size,
        total_pages: total_items.div_ceil(pagination.page_size),
        total_items,
    };
    Ok(json(
        StatusCode::OK,
        page.into_iter().map(Role::from).collect::<Vec<_>>(),
        PageListMeta {
            pagination: PagePagination::new(&meta, &uri),
        },
    ))
}

/// `POST /v1/iam/roles`: make a custom role.
///
/// # Errors
///
/// `403` without the permission to manage roles, `422` when the role to clone
/// is not one of the company's, and a refusal when the role breaks a role rule.
#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    session: AuthSession,
    ValidatedJson(req): ValidatedJson<CreateRoleRequest>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let new = req.into_domain();
    let detail = roles::Model::create_custom(
        &ctx.db,
        access.tenant_id(),
        &new.name,
        new.description,
        new.permissions,
        new.clone_from.as_deref(),
        &access.permissions(),
    )
    .await?
    .map_err(|err| match err {
        RoleError::NotFound => Problem::unprocessable(
            "unknown_role",
            "`clone_from_role_id` is not a role of this company.",
        )
        .field("clone_from_role_id"),
        other => role_problem(&other),
    })?;
    Ok(json(
        StatusCode::CREATED,
        Role::from(detail),
        EmptyMeta::default(),
    ))
}

/// `GET /v1/iam/roles/{id}`
///
/// # Errors
///
/// `403` without the permission to read roles, and `404` when the company has
/// no such role.
#[debug_handler]
pub async fn show(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult {
    let access = session.authorize(&ctx, &READ).await?;
    let role = find(&ctx, access.tenant_id(), &id).await?;
    let detail = role.detail(&ctx.db, access.tenant_id()).await?;
    Ok(json(
        StatusCode::OK,
        Role::from(detail),
        EmptyMeta::default(),
    ))
}

/// `PATCH /v1/iam/roles/{id}`: change a custom role.
///
/// # Errors
///
/// `403` without the permission to manage roles, `404` for an unknown role, and
/// a refusal for an invalid change or one a role rule forbids.
#[debug_handler]
pub async fn update(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<UpdateRoleRequest>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let changes = req.into_changes()?;
    let role = find(&ctx, access.tenant_id(), &id).await?;
    let detail = role
        .update_custom(&ctx.db, access.tenant_id(), changes, &access.permissions())
        .await?
        .map_err(|err| role_problem(&err))?;
    Ok(json(
        StatusCode::OK,
        Role::from(detail),
        EmptyMeta::default(),
    ))
}

/// `DELETE /v1/iam/roles/{id}`: delete a custom role nobody holds.
///
/// # Errors
///
/// `403` without the permission to manage roles, `404` for an unknown role, and
/// a refusal when the role cannot be deleted.
#[debug_handler]
pub async fn remove(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let role = find(&ctx, access.tenant_id(), &id).await?;
    role.delete_custom(&ctx.db, access.tenant_id())
        .await?
        .map_err(|err| role_problem(&err))?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
