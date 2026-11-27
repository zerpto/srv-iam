use axum::http::{HeaderMap, StatusCode};
use loco_rs::prelude::*;
use zerpto_core::{
    helpers::json,
    http::{
        envelope::EmptyMeta,
        idempotency::IdempotencyKey,
        ids::ApiIds,
        problem::{ApiResult, Problem},
    },
    validation::ValidatedJson,
};

use crate::{
    grpc::regions,
    http::{
        auth_session::AuthSession,
        ids::TENANT,
        v1::{
            requests::tenants::{CreateTenantRequest, SwitchTenantRequest, UpdateTenantRequest},
            response::tenants::{Membership, Tenant, TenantSession, TenantSummary},
        },
    },
    models::{
        address::{Address, AddressError, AddressInput},
        memberships,
        scope::Scope,
        sessions,
        tenants::{self, InputError},
        users,
    },
};

/// Permission needed to change the tenant's profile and settings.
const MANAGE: [&str; 1] = ["iam.tenant.manage"];

fn input_problem(err: &InputError) -> Problem {
    match err {
        InputError::TaxIdTaken => {
            Problem::conflict("tax_id_taken", "A tenant with this tax id already exists.")
        }
    }
}

fn address_problem(err: &AddressError) -> Problem {
    match err {
        AddressError::UnknownRegion => {
            Problem::unprocessable("unknown_region", "This region does not exist.")
        }
        AddressError::RegionRequired => Problem::unprocessable(
            "region_required",
            "Pick a region for this country; its address cannot be typed by hand.",
        ),
        AddressError::RegionTooShallow => Problem::unprocessable(
            "region_too_shallow",
            "Pick a region deeper in the hierarchy for this country.",
        ),
        AddressError::CountryRequired => Problem::unprocessable(
            "country_required",
            "An address without a region needs a valid country code.",
        ),
    }
}

/// Moves the session of `user` to the tenant of `membership`: the session they
/// called with ends (the contract revokes the previous refresh token) and a
/// new one scoped to the tenant starts.
async fn switch_to(
    ctx: &AppContext,
    session: &AuthSession,
    user: &users::Model,
    membership: memberships::Model,
) -> Result<TenantSession, Problem> {
    let membership = membership.into_active_model().touch(&ctx.db).await?;
    let scope = Scope::load(&ctx.db, membership).await?;
    sessions::revoke(ctx, &session.sid).await?;
    let tokens = sessions::start(ctx, user, Some(&scope)).await?;
    Ok(TenantSession {
        tokens: tokens.into(),
        tenant: TenantSummary::from(&scope),
    })
}

/// Has reference-data check the address, when the body has one.
async fn resolve_address(
    ctx: &AppContext,
    input: Option<AddressInput>,
) -> Result<Option<Address>, Problem> {
    let Some(input) = input else {
        return Ok(None);
    };
    regions::resolve(ctx, input)
        .await?
        .map(Some)
        .map_err(|err| address_problem(&err))
}

/// `GET /v1/iam/tenant`: the profile and settings of the session's tenant.
///
/// # Errors
///
/// Fails when the session has no access to a company or it cannot be loaded.
#[debug_handler]
pub async fn show(State(ctx): State<AppContext>, session: AuthSession) -> ApiResult {
    let access = session.access(&ctx).await?;
    Ok(json(
        StatusCode::OK,
        Tenant::from(access.scope.tenant),
        EmptyMeta::default(),
    ))
}

/// `PATCH /v1/iam/tenant`: change only the fields sent.
///
/// # Errors
///
/// `403` without the permission to manage the company, and a refusal for
/// invalid input. Fails too when the address cannot be checked or the change
/// cannot be saved.
#[debug_handler]
pub async fn update(
    State(ctx): State<AppContext>,
    session: AuthSession,
    ValidatedJson(req): ValidatedJson<UpdateTenantRequest>,
) -> ApiResult {
    let access = session.authorize(&ctx, &MANAGE).await?;
    let address = resolve_address(&ctx, req.address_input()).await?;
    let tenant = access
        .scope
        .tenant
        .apply(&ctx.db, req.into_domain(address))
        .await?
        .map_err(|err| input_problem(&err))?;
    Ok(json(
        StatusCode::OK,
        Tenant::from(tenant),
        EmptyMeta::default(),
    ))
}

/// `GET /v1/iam/tenants`: every tenant the caller belongs to.
///
/// # Errors
///
/// Fails when the user or their memberships cannot be loaded.
#[debug_handler]
pub async fn index(State(ctx): State<AppContext>, session: AuthSession) -> ApiResult {
    let user = session.user(&ctx).await?;
    let mut list = Vec::new();
    for membership in memberships::Model::list_for_user(&ctx.db, user.id).await? {
        let is_active = session.tenant_id() == Some(membership.tenant_id);
        let scope = Scope::load(&ctx.db, membership).await?;
        list.push(Membership::new(
            &scope.membership,
            TenantSummary::from(&scope),
            is_active,
        ));
    }
    Ok(json(StatusCode::OK, list, EmptyMeta::default()))
}

/// The caller's membership of `tenant`; a missing one is a bug for a tenant
/// the caller just created.
async fn membership_of(
    ctx: &AppContext,
    user_id: uuid::Uuid,
    tenant: &tenants::Model,
) -> Result<memberships::Model, Problem> {
    memberships::Model::find_in_tenant(&ctx.db, user_id, tenant.id)
        .await?
        .ok_or_else(|| Problem::from(Error::string("owner membership missing")))
}

/// `POST /v1/iam/tenants`: create a company and make the caller its owner.
///
/// Returns a token pair scoped to the new tenant. With an `Idempotency-Key`, a
/// retry returns the same tenant (with fresh tokens) instead of a second one.
///
/// # Errors
///
/// Refuses invalid input. Fails too when the address cannot be checked, the
/// company cannot be saved or the session cannot be moved to it.
#[debug_handler]
pub async fn create(
    State(ctx): State<AppContext>,
    session: AuthSession,
    headers: HeaderMap,
    ValidatedJson(req): ValidatedJson<CreateTenantRequest>,
) -> ApiResult {
    let user = session.user(&ctx).await?;

    let key = IdempotencyKey::from_headers(&headers, "iam.tenants", &user.id.to_string())?;
    if let Some(key) = &key {
        if let Some(id) = key.recall(&ctx).await? {
            if let Ok(id) = uuid::Uuid::parse_str(&id) {
                if let Ok(tenant) = tenants::Model::find_by_id(&ctx.db, id).await {
                    let membership = membership_of(&ctx, user.id, &tenant).await?;
                    let replay = switch_to(&ctx, &session, &user, membership).await?;
                    return Ok(json(StatusCode::CREATED, replay, EmptyMeta::default()));
                }
            }
        }
    }

    let address = resolve_address(&ctx, req.address_input()).await?;
    let tenant = tenants::Model::create_with_owner(&ctx.db, user.id, req.into_domain(address))
        .await?
        .map_err(|err| input_problem(&err))?;
    if let Some(key) = &key {
        key.remember(&ctx, &tenant.id.to_string()).await?;
    }

    let membership = membership_of(&ctx, user.id, &tenant).await?;
    let created = switch_to(&ctx, &session, &user, membership).await?;
    Ok(json(StatusCode::CREATED, created, EmptyMeta::default()))
}

/// `POST /v1/auth/switch-tenant`: move the session to another tenant the
/// caller is an active member of. The session they called with ends (the
/// contract revokes the previous refresh token).
///
/// # Errors
///
/// `400` for a malformed tenant id, `404` for an unknown tenant and `403` when
/// the caller is not an active member of it. Fails too when the session cannot
/// be moved.
#[debug_handler]
pub async fn switch_tenant(
    State(ctx): State<AppContext>,
    session: AuthSession,
    ValidatedJson(req): ValidatedJson<SwitchTenantRequest>,
) -> ApiResult {
    let user = session.user(&ctx).await?;
    let tenant_id = ApiIds::new(TENANT).parse(&req.tenant_id).ok_or_else(|| {
        Problem::bad_request("invalid_tenant_id", "This is not a tenant id.").field("tenant_id")
    })?;
    let tenant = tenants::Model::find_by_id(&ctx.db, tenant_id)
        .await
        .map_err(|_| Problem::not_found("This tenant does not exist."))?;
    // A deactivated member is not a member for this purpose.
    let membership = memberships::Model::find_in_tenant(&ctx.db, user.id, tenant.id)
        .await?
        .filter(memberships::Model::is_active)
        .ok_or_else(|| Problem::forbidden("not_a_member", "You do not belong to this tenant."))?;

    let switched = switch_to(&ctx, &session, &user, membership).await?;
    Ok(json(StatusCode::OK, switched, EmptyMeta::default()))
}
