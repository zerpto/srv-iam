//! The people of a tenant (`User` in the API contract): members, who have a
//! membership, and invitees, who have a pending invitation.
//!
//! Both have a `mem-` id, and an invitation keeps its id when it is accepted.
//!
//! An invitation or a reactivation is refused when every seat is taken
//! (`SeatLimitReached`). The limit belongs to the tenant's plan: the caller
//! asks the plan provider for it before calling and passes it in, so no network
//! call runs here.

use std::collections::HashMap;

use chrono::{DateTime, Duration, Utc};
use loco_rs::{model::query::PaginationQuery, prelude::*};
use sea_orm::{Condition, PaginatorTrait, TransactionTrait};
use uuid::Uuid;
use zerpto_core::http::ids::ApiIds;

use super::{
    _entities::{invitations, membership_roles, memberships, roles, users},
    digest::sha256_hex,
    roles as role_rules,
};
use crate::http::ids::{MEMBERSHIP, ROLE};

pub const INVITED: &str = "invited";
pub const ACTIVE: &str = "active";
pub const DEACTIVATED: &str = "deactivated";

/// How long an invitation link works.
pub const INVITATION_VALID_DAYS: i64 = 7;

/// A person in a tenant, as the API shows them.
#[derive(Debug, Clone)]
pub struct Member {
    /// The membership's id, or the invitation's until it is accepted.
    pub id: Uuid,
    pub email: String,
    /// `None` until the person accepts and types their name.
    pub name: Option<String>,
    /// API ids (`rol-<uuid>`), sorted.
    pub role_ids: Vec<String>,
    pub status: &'static str,
    pub created_at: DateTime<Utc>,
}

impl Member {
    /// The API id: `mem-<uuid>`.
    #[must_use]
    pub fn api_id(&self) -> String {
        ApiIds::new(MEMBERSHIP).encode(&self.id)
    }
}

/// Why a change to a person was refused.
#[derive(Debug, PartialEq, Eq)]
pub enum MemberError {
    NotFound,
    /// The email already belongs to an active member of the tenant.
    AlreadyMember,
    /// The person is a member of the tenant but deactivated.
    UserDeactivated,
    /// A role id is not a role of this tenant.
    UnknownRole,
    /// The change would leave the tenant without an active owner.
    LastOwner,
    /// Only an invitation can be revoked.
    NotInvited,
    /// The person has not accepted their invitation yet.
    Invited,
    /// Every seat of the plan is taken.
    SeatLimitReached,
}

type Outcome<T> = ModelResult<Result<T, MemberError>>;

/// What to leave out of a list.
#[derive(Debug, Default)]
pub struct Filter {
    pub created_after: Option<DateTime<Utc>>,
    pub created_before: Option<DateTime<Utc>>,
}

/// One page of people, plus the numbers for `meta.pagination`.
#[derive(Debug)]
pub struct Page {
    pub items: Vec<Member>,
    pub page: u64,
    pub page_size: u64,
    pub total_items: u64,
    pub total_pages: u64,
}

/// The record behind a person.
enum Target {
    Member(memberships::Model, users::Model),
    Invitee(invitations::Model),
}

fn api_role_ids(ids: &[Uuid]) -> Vec<String> {
    let roles = ApiIds::new(ROLE);
    let mut api: Vec<String> = ids.iter().map(|id| roles.encode(id)).collect();
    api.sort();
    api
}

/// The role ids held through each of `membership_ids`.
async fn roles_of<C: ConnectionTrait>(
    db: &C,
    membership_ids: Vec<Uuid>,
) -> ModelResult<HashMap<Uuid, Vec<Uuid>>> {
    let mut held: HashMap<Uuid, Vec<Uuid>> = HashMap::new();
    for link in membership_roles::Entity::find()
        .filter(membership_roles::Column::MembershipId.is_in(membership_ids))
        .all(db)
        .await?
    {
        held.entry(link.membership_id)
            .or_default()
            .push(link.role_id);
    }
    Ok(held)
}

const fn member_status(membership: &memberships::Model) -> &'static str {
    if membership.deactivated_at.is_some() {
        DEACTIVATED
    } else {
        ACTIVE
    }
}

fn member_of(
    membership: &memberships::Model,
    user: &users::Model,
    held: &HashMap<Uuid, Vec<Uuid>>,
) -> Member {
    Member {
        id: membership.id,
        email: user.email.clone(),
        name: Some(user.name.clone()),
        role_ids: api_role_ids(held.get(&membership.id).map_or(&[], Vec::as_slice)),
        status: member_status(membership),
        created_at: membership.created_at.with_timezone(&Utc),
    }
}

fn invitee_of(invitation: &invitations::Model) -> Member {
    Member {
        id: invitation.id,
        email: invitation.email.clone(),
        name: None,
        role_ids: api_role_ids(&invitation.role_uuids()),
        status: INVITED,
        created_at: invitation.created_at.with_timezone(&Utc),
    }
}

/// The roles for `api_ids`, which must all be built-in roles or roles of the
/// tenant.
async fn resolve_roles<C: ConnectionTrait>(
    db: &C,
    tenant_id: Uuid,
    api_ids: &[String],
) -> ModelResult<Result<Vec<roles::Model>, MemberError>> {
    let api = ApiIds::new(ROLE);
    let Some(mut wanted) = api_ids
        .iter()
        .map(|id| api.parse(id))
        .collect::<Option<Vec<Uuid>>>()
    else {
        return Ok(Err(MemberError::UnknownRole));
    };
    wanted.sort();
    wanted.dedup();
    if wanted.is_empty() {
        return Ok(Err(MemberError::UnknownRole));
    }
    let found = roles::Entity::find()
        .filter(roles::Column::Id.is_in(wanted.clone()))
        .filter(
            Condition::any()
                .add(roles::Column::TenantId.is_null())
                .add(roles::Column::TenantId.eq(tenant_id)),
        )
        .all(db)
        .await?;
    if found.len() == wanted.len() {
        Ok(Ok(found))
    } else {
        Ok(Err(MemberError::UnknownRole))
    }
}

/// Whether some active member other than `except` holds the owner role.
async fn has_other_owner<C: ConnectionTrait>(
    db: &C,
    tenant_id: Uuid,
    except: Uuid,
) -> ModelResult<bool> {
    let Some(owner) = role_rules::Model::find_system(db, role_rules::OWNER).await? else {
        return Ok(false);
    };
    let active: Vec<Uuid> = memberships::Entity::find()
        .filter(memberships::Column::TenantId.eq(tenant_id))
        .filter(memberships::Column::DeactivatedAt.is_null())
        .filter(memberships::Column::Id.ne(except))
        .all(db)
        .await?
        .into_iter()
        .map(|m| m.id)
        .collect();
    Ok(membership_roles::Entity::find()
        .filter(membership_roles::Column::RoleId.eq(owner.id))
        .filter(membership_roles::Column::MembershipId.is_in(active))
        .count(db)
        .await?
        > 0)
}

/// Whether the membership holds the owner role.
async fn is_owner<C: ConnectionTrait>(db: &C, membership_id: Uuid) -> ModelResult<bool> {
    let Some(owner) = role_rules::Model::find_system(db, role_rules::OWNER).await? else {
        return Ok(false);
    };
    Ok(membership_roles::Entity::find()
        .filter(membership_roles::Column::RoleId.eq(owner.id))
        .filter(membership_roles::Column::MembershipId.eq(membership_id))
        .count(db)
        .await?
        > 0)
}

async fn locate<C: ConnectionTrait>(
    db: &C,
    tenant_id: Uuid,
    api_id: &str,
) -> ModelResult<Option<Target>> {
    let Some(id) = ApiIds::new(MEMBERSHIP).parse(api_id) else {
        return Ok(None);
    };
    if let Some(membership) = memberships::Entity::find()
        .filter(memberships::Column::TenantId.eq(tenant_id))
        .filter(memberships::Column::Id.eq(id))
        .one(db)
        .await?
    {
        let user = users::Entity::find_by_id(membership.user_id)
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)?;
        return Ok(Some(Target::Member(membership, user)));
    }
    Ok(invitations::Entity::find()
        .filter(invitations::Column::TenantId.eq(tenant_id))
        .filter(invitations::Column::Id.eq(id))
        .filter(invitations::Column::AcceptedAt.is_null())
        .one(db)
        .await?
        .map(Target::Invitee))
}

async fn present<C: ConnectionTrait>(db: &C, target: &Target) -> ModelResult<Member> {
    Ok(match target {
        Target::Member(membership, user) => {
            let held = roles_of(db, vec![membership.id]).await?;
            member_of(membership, user, &held)
        }
        Target::Invitee(invitation) => invitee_of(invitation),
    })
}

/// The tenant's people (members of every status, and pending invitations),
/// sorted by creation time.
///
/// # Errors
///
/// When a query fails.
pub async fn list<C: ConnectionTrait>(
    db: &C,
    tenant_id: Uuid,
    filter: &Filter,
    descending: bool,
    paging: &PaginationQuery,
) -> ModelResult<Page> {
    let memberships = memberships::Entity::find()
        .filter(memberships::Column::TenantId.eq(tenant_id))
        .all(db)
        .await?;
    let users: HashMap<Uuid, users::Model> = users::Entity::find()
        .filter(users::Column::Id.is_in(memberships.iter().map(|m| m.user_id)))
        .all(db)
        .await?
        .into_iter()
        .map(|user| (user.id, user))
        .collect();
    let held = roles_of(db, memberships.iter().map(|m| m.id).collect()).await?;
    let pending = invitations::Entity::find()
        .filter(invitations::Column::TenantId.eq(tenant_id))
        .filter(invitations::Column::AcceptedAt.is_null())
        .all(db)
        .await?;

    let mut people: Vec<Member> = memberships
        .iter()
        .filter_map(|membership| {
            users
                .get(&membership.user_id)
                .map(|user| member_of(membership, user, &held))
        })
        .collect();
    people.extend(pending.iter().map(invitee_of));

    people.retain(|person| {
        filter
            .created_after
            .is_none_or(|after| person.created_at > after)
            && filter
                .created_before
                .is_none_or(|before| person.created_at < before)
    });
    // UUIDv7 ids sort by creation time, so `id` breaks ties.
    people.sort_by_key(|a| (a.created_at, a.id));
    if descending {
        people.reverse();
    }

    let total_items = people.len() as u64;
    let page_size = paging.page_size;
    let total_pages = total_items.div_ceil(page_size);
    let skip = usize::try_from((paging.page - 1).saturating_mul(page_size)).unwrap_or(usize::MAX);
    let items = people
        .into_iter()
        .skip(skip)
        .take(usize::try_from(page_size).unwrap_or(usize::MAX))
        .collect();
    Ok(Page {
        items,
        page: paging.page,
        page_size,
        total_items,
        total_pages,
    })
}

/// One person of the tenant.
///
/// # Errors
///
/// When a query fails.
pub async fn find<C: ConnectionTrait>(
    db: &C,
    tenant_id: Uuid,
    api_id: &str,
) -> ModelResult<Option<Member>> {
    match locate(db, tenant_id, api_id).await? {
        Some(target) => Ok(Some(present(db, &target).await?)),
        None => Ok(None),
    }
}

/// Seats in use: every active member and every pending invitation, except the
/// one for `except_email` (about to be replaced).
///
/// # Errors
///
/// When a query fails.
pub async fn seats_in_use<C: ConnectionTrait>(
    db: &C,
    tenant_id: Uuid,
    except_email: Option<&str>,
) -> ModelResult<i64> {
    let active = memberships::Entity::find()
        .filter(memberships::Column::TenantId.eq(tenant_id))
        .filter(memberships::Column::DeactivatedAt.is_null())
        .count(db)
        .await?;
    let mut pending = invitations::Entity::find()
        .filter(invitations::Column::TenantId.eq(tenant_id))
        .filter(invitations::Column::AcceptedAt.is_null());
    if let Some(email) = except_email {
        pending = pending.filter(invitations::Column::Email.ne(email));
    }
    let pending = pending.count(db).await?;
    Ok(i64::try_from(active + pending).unwrap_or(i64::MAX))
}

/// An invitation that was just created, with the secret for its link.
#[derive(Debug)]
pub struct Invited {
    pub member: Member,
    /// Goes into the email only; just its hash is stored.
    pub token: String,
}

/// Invites `email` to the tenant with the given roles. Inviting an address
/// that has a pending invitation replaces it, so the old link stops working.
///
/// # Errors
///
/// `AlreadyMember`, `UserDeactivated` or `UnknownRole`; the outer error is a
/// failed query.
pub async fn invite(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    email: &str,
    role_api_ids: &[String],
    seat_limit: i64,
) -> Outcome<Invited> {
    let roles = match resolve_roles(db, tenant_id, role_api_ids).await? {
        Ok(roles) => roles,
        Err(err) => return Ok(Err(err)),
    };
    if let Some(user) = users::Entity::find()
        .filter(users::Column::Email.eq(email))
        .one(db)
        .await?
    {
        if let Some(membership) =
            super::memberships::Model::find_in_tenant(db, user.id, tenant_id).await?
        {
            return Ok(Err(if membership.is_active() {
                MemberError::AlreadyMember
            } else {
                MemberError::UserDeactivated
            }));
        }
    }

    // Inviting the same address again replaces its invitation, so that one
    // does not count.
    if seats_in_use(db, tenant_id, Some(email)).await? >= seat_limit {
        return Ok(Err(MemberError::SeatLimitReached));
    }

    let token = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let txn = db.begin().await?;
    invitations::Entity::delete_many()
        .filter(invitations::Column::TenantId.eq(tenant_id))
        .filter(invitations::Column::Email.eq(email))
        .filter(invitations::Column::AcceptedAt.is_null())
        .exec(&txn)
        .await?;
    let invitation = invitations::ActiveModel {
        tenant_id: ActiveValue::set(tenant_id),
        email: ActiveValue::set(email.to_string()),
        name: ActiveValue::set(None),
        token_hash: ActiveValue::set(sha256_hex(&token)),
        role_ids: ActiveValue::set(serde_json::json!(roles
            .iter()
            .map(|role| role.id)
            .collect::<Vec<_>>())),
        expires_at: ActiveValue::set((Utc::now() + Duration::days(INVITATION_VALID_DAYS)).into()),
        ..Default::default()
    }
    .insert(&txn)
    .await?;
    txn.commit().await?;

    Ok(Ok(Invited {
        member: invitee_of(&invitation),
        token,
    }))
}

/// Replaces the person's roles.
///
/// # Errors
///
/// `NotFound`, `UserDeactivated`, `UnknownRole` or `LastOwner`.
pub async fn change_roles(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    api_id: &str,
    role_api_ids: &[String],
) -> Outcome<Member> {
    let Some(target) = locate(db, tenant_id, api_id).await? else {
        return Ok(Err(MemberError::NotFound));
    };
    let roles = match resolve_roles(db, tenant_id, role_api_ids).await? {
        Ok(roles) => roles,
        Err(err) => return Ok(Err(err)),
    };
    let role_ids: Vec<Uuid> = roles.iter().map(|role| role.id).collect();
    match &target {
        Target::Invitee(invitation) => {
            let mut changed = invitation.clone().into_active_model();
            changed.role_ids = ActiveValue::set(serde_json::json!(role_ids));
            let updated = changed.update(db).await?;
            Ok(Ok(invitee_of(&updated)))
        }
        Target::Member(membership, _) => {
            if !membership.is_active() {
                return Ok(Err(MemberError::UserDeactivated));
            }
            let keeps_owner = roles
                .iter()
                .any(|role| role.tenant_id.is_none() && role.name == role_rules::OWNER);
            if !keeps_owner
                && is_owner(db, membership.id).await?
                && !has_other_owner(db, tenant_id, membership.id).await?
            {
                return Ok(Err(MemberError::LastOwner));
            }
            let txn = db.begin().await?;
            membership.replace_roles(&txn, &role_ids).await?;
            txn.commit().await?;
            Ok(Ok(present(db, &target).await?))
        }
    }
}

/// Cancels an invitation nobody accepted yet.
///
/// # Errors
///
/// `NotFound`, or `NotInvited` for someone who already accepted.
pub async fn revoke(db: &DatabaseConnection, tenant_id: Uuid, api_id: &str) -> Outcome<()> {
    match locate(db, tenant_id, api_id).await? {
        None => Ok(Err(MemberError::NotFound)),
        Some(Target::Member(..)) => Ok(Err(MemberError::NotInvited)),
        Some(Target::Invitee(invitation)) => {
            invitation.delete(db).await?;
            Ok(Ok(()))
        }
    }
}

/// Takes the person's access to the tenant away. Deactivating someone who is
/// already deactivated changes nothing.
///
/// # Errors
///
/// `NotFound`, `Invited` or `LastOwner`.
pub async fn deactivate(db: &DatabaseConnection, tenant_id: Uuid, api_id: &str) -> Outcome<()> {
    match locate(db, tenant_id, api_id).await? {
        None => Ok(Err(MemberError::NotFound)),
        Some(Target::Invitee(_)) => Ok(Err(MemberError::Invited)),
        Some(Target::Member(membership, _)) => {
            if !membership.is_active() {
                return Ok(Ok(()));
            }
            if is_owner(db, membership.id).await?
                && !has_other_owner(db, tenant_id, membership.id).await?
            {
                return Ok(Err(MemberError::LastOwner));
            }
            let mut changed = membership.into_active_model();
            changed.deactivated_at = ActiveValue::set(Some(Utc::now().into()));
            changed.update(db).await?;
            Ok(Ok(()))
        }
    }
}

/// Gives a deactivated person their access back. Reactivating someone who is
/// already active changes nothing.
///
/// # Errors
///
/// `NotFound` or `Invited`.
pub async fn reactivate(
    db: &DatabaseConnection,
    tenant_id: Uuid,
    api_id: &str,
    seat_limit: i64,
) -> Outcome<Member> {
    match locate(db, tenant_id, api_id).await? {
        None => Ok(Err(MemberError::NotFound)),
        Some(Target::Invitee(_)) => Ok(Err(MemberError::Invited)),
        Some(Target::Member(membership, user)) => {
            let membership = if membership.is_active() {
                membership
            } else {
                if seats_in_use(db, tenant_id, None).await? >= seat_limit {
                    return Ok(Err(MemberError::SeatLimitReached));
                }
                let mut changed = membership.into_active_model();
                changed.deactivated_at = ActiveValue::set(None);
                changed.update(db).await?
            };
            Ok(Ok(present(db, &Target::Member(membership, user)).await?))
        }
    }
}
