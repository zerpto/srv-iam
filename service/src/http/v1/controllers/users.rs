use axum::http::{HeaderMap, StatusCode, Uri};
use loco_rs::{controller::views::pagination::PagerMeta, prelude::*};
use serde::Deserialize;
use serde_json::json;
use zerpto_core::{
    helpers::json,
    http::{
        envelope::{EmptyMeta, PageListMeta, PagePagination},
        filters::QueryParam,
        idempotency::IdempotencyKey,
        paging::Paging,
        problem::{ApiResult, Problem},
    },
    relay::message::RelayMessage,
    validation::ValidatedJson,
};

use crate::{
    http::{
        auth_session::{Access, AuthSession},
        v1::{
            requests::{
                users::{InviteUserRequest, UpdateUserRequest},
                validation::rules::normalize_email,
            },
            response::users::User,
        },
    },
    models::{
        members::{self, Filter, Member, MemberError},
        roles,
    },
    settings::Settings,
};

const READ: [&str; 2] = ["iam.users.read", "iam.users.manage"];
const MANAGE: [&str; 1] = ["iam.users.manage"];

fn member_problem(err: &MemberError) -> Problem {
    match err {
        MemberError::NotFound => Problem::not_found("No such user."),
        MemberError::AlreadyMember => Problem::conflict(
            "already_member",
            "This person already belongs to the company.",
        )
        .field("email"),
        MemberError::UserDeactivated => Problem::conflict(
            "user_deactivated",
            "This user is deactivated. Reactivate them first.",
        ),
        MemberError::UnknownRole => {
            Problem::unprocessable("unknown_role", "Every role must be a role of this company.")
                .field("role_ids")
        }
        MemberError::LastOwner => Problem::conflict(
            "last_owner",
            "The company needs at least one active owner. Make someone else an owner first.",
        ),
        MemberError::NotInvited => Problem::conflict(
            "user_not_invited",
            "Only an invitation that was not accepted can be revoked.",
        ),
        MemberError::SeatLimitReached => Problem::conflict(
            "seat_limit_reached",
            "Every seat of the plan is taken. Buy more seats under Billing first.",
        ),
        MemberError::Invited => Problem::conflict(
            "user_invited",
            "This user has not accepted the invitation yet.",
        ),
    }
}

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    sort: Option<String>,
    created_after: Option<String>,
    created_before: Option<String>,
}

impl ListQuery {
    fn resolve(&self) -> Result<(Filter, bool), Problem> {
        let descending = match self.sort.as_deref() {
            None | Some("created_at") => false,
            Some("-created_at") => true,
            Some(_) => {
                return Err(Problem::bad_request(
                    "invalid_sort",
                    "`sort` must be `created_at`, with `-` in front for descending.",
                )
                .field("sort"))
            }
        };
        let filter = Filter {
            created_after: QueryParam::new(
                self.created_after.as_deref(),
                "created_after",
                "invalid_created_after",
            )
            .timestamp()?,
            created_before: QueryParam::new(
                self.created_before.as_deref(),
                "created_before",
                "invalid_created_before",
            )
            .timestamp()?,
        };
        Ok((filter, descending))
    }
}

async fn show_user(ctx: &AppContext, access: &Access, id: &str) -> Result<User, Problem> {
    let member = members::find(&ctx.db, access.tenant_id(), id)
        .await?
        .ok_or_else(|| member_problem(&MemberError::NotFound))?;
    render_user(ctx, access, member).await
}

/// Builds the API user, with `access` from the tenant's plan.
async fn render_user(ctx: &AppContext, access: &Access, member: Member) -> Result<User, Problem> {
    let owner_role_id = roles::Model::owner_api_id(&ctx.db).await?;
    Ok(User::new(member, access.plan.locked, &owner_role_id))
}

/// `GET /v1/iam/users`: the company's people, invitations included.
///
/// # Errors
///
/// `400` for an invalid filter or paging, and `403` without the permission to
/// read users.
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
    let (filter, descending) = query.resolve()?;
    let page = members::list(
        &ctx.db,
        access.tenant_id(),
        &filter,
        descending,
        &pagination,
    )
    .await?;
    let meta = PagerMeta {
        page: page.page,
        page_size: page.page_size,
        total_pages: page.total_pages,
        total_items: page.total_items,
    };
    let owner_role_id = roles::Model::owner_api_id(&ctx.db).await?;
    let locked = access.plan.locked;
    Ok(json(
        StatusCode::OK,
        page.items
            .into_iter()
            .map(|member| User::new(member, locked, &owner_role_id))
            .collect::<Vec<_>>(),
        PageListMeta {
            pagination: PagePagination::new(&meta, &uri),
        },
    ))
}

/// `POST /v1/iam/users`: invite a person by email. With an `Idempotency-Key`,
/// a retry returns the same invitation instead of inviting again.
///
/// # Errors
///
/// `403` without the permission to manage users, and a refusal when the person
/// cannot be invited, for example when every seat is taken. A message that
/// cannot be queued is only logged.
#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    session: AuthSession,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<InviteUserRequest>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let email = normalize_email(&req.email);

    let key = IdempotencyKey::from_headers(&headers, "iam.users", &access.tenant_id().to_string())?;
    if let Some(key) = &key {
        if let Some(id) = key.recall(&ctx).await? {
            if let Ok(user) = show_user(&ctx, &access, &id).await {
                return Ok(json(StatusCode::CREATED, user, EmptyMeta::default()));
            }
        }
    }

    let invited = members::invite(
        &ctx.db,
        access.tenant_id(),
        &email,
        &req.role_ids,
        access.plan.seat_limit,
    )
    .await?
    .map_err(|err| member_problem(&err))?;
    // The invitation exists either way: a message that fails to queue can be
    // sent again by inviting the address once more.
    let accept_url = format!(
        "{}?token={}",
        Settings::load(&ctx)?.url("accept-invite"),
        invited.token
    );
    if let Err(err) = (RelayMessage {
        channel: "email".to_string(),
        recipient: invited.member.email.clone(),
        template: "auth.invitation".to_string(),
        tenant_id: Some(access.scope.tenant.api_id()),
        data: json!({
            "company": access.scope.tenant.name,
            "accept_url": accept_url,
            "expires_in_days": members::INVITATION_VALID_DAYS,
        }),
    })
    .send(&ctx)
    .await
    {
        tracing::error!(error = %err, "could not queue the invitation email");
    }
    if let Some(key) = &key {
        key.remember(&ctx, &invited.member.api_id()).await?;
    }
    Ok(json(
        StatusCode::CREATED,
        render_user(&ctx, &access, invited.member).await?,
        EmptyMeta::default(),
    ))
}

/// `GET /v1/iam/users/{id}`
///
/// # Errors
///
/// `403` without the permission to read users, and `404` when the company has
/// no such user.
#[debug_handler]
pub async fn show(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult {
    let access = session.authorize(&ctx, &READ).await?;
    Ok(json(
        StatusCode::OK,
        show_user(&ctx, &access, &id).await?,
        EmptyMeta::default(),
    ))
}

/// `PATCH /v1/iam/users/{id}`: replace the user's roles.
///
/// # Errors
///
/// `403` without the permission to manage users, and a refusal when the roles
/// cannot be changed.
#[debug_handler]
pub async fn update(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
    ValidatedJson(req): ValidatedJson<UpdateUserRequest>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let member = members::change_roles(&ctx.db, access.tenant_id(), &id, &req.role_ids)
        .await?
        .map_err(|err| member_problem(&err))?;
    Ok(json(
        StatusCode::OK,
        render_user(&ctx, &access, member).await?,
        EmptyMeta::default(),
    ))
}

/// `DELETE /v1/iam/users/{id}`: cancel an invitation nobody accepted yet.
///
/// # Errors
///
/// `403` without the permission to manage users, and a refusal when the
/// invitation cannot be cancelled.
#[debug_handler]
pub async fn revoke(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    members::revoke(&ctx.db, access.tenant_id(), &id)
        .await?
        .map_err(|err| member_problem(&err))?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

/// `PUT /v1/iam/users/{id}/activate`
///
/// # Errors
///
/// `403` without the permission to manage users, and a refusal when the user
/// cannot be reactivated, for example when every seat is taken.
#[debug_handler]
pub async fn reactivate(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let member = members::reactivate(&ctx.db, access.tenant_id(), &id, access.plan.seat_limit)
        .await?
        .map_err(|err| member_problem(&err))?;
    Ok(json(
        StatusCode::OK,
        render_user(&ctx, &access, member).await?,
        EmptyMeta::default(),
    ))
}

/// `DELETE /v1/iam/users/{id}/activate`
///
/// # Errors
///
/// `403` without the permission to manage users, and a refusal when the user
/// cannot be deactivated.
#[debug_handler]
pub async fn deactivate(
    State(ctx): State<AppContext>,
    session: AuthSession,
    Path(id): Path<String>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    members::deactivate(&ctx.db, access.tenant_id(), &id)
        .await?
        .map_err(|err| member_problem(&err))?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
