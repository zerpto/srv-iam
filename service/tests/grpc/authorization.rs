use loco_rs::{app::AppContext, testing::prelude::*};
use sea_orm::{ActiveModelTrait, ActiveValue, IntoActiveModel};
use serial_test::serial;
use tonic::{Code, Request};
use uuid::Uuid;
use zerpto_iam::{
    app::App,
    grpc::authorization::AuthorizationService,
    models::{
        memberships,
        tenants::{self, NewTenant},
        users::{self, NewUser},
    },
};
use zerpto_proto::iam::v1::{authorization_server::Authorization, AuthorizeRequest, Decision};

/// A new tenant and its owner.
async fn owner_and_tenant(ctx: &AppContext) -> (users::Model, tenants::Model) {
    let user = users::Model::create_with_password(
        &ctx.db,
        &NewUser {
            email: "owner@example.com",
            name: "Owner",
            password: "a-long-test-password",
            email_verified: true,
        },
    )
    .await
    .unwrap();
    let tenant = tenants::Model::create_with_owner(
        &ctx.db,
        user.id,
        NewTenant {
            name: "Acme".to_string(),
            legal_name: None,
            tax_id: None,
            base_currency: "IDR".to_string(),
            timezone: "Asia/Jakarta".to_string(),
            fiscal_year_start_month: 1,
            completeness: "minimal".to_string(),
            address: None,
        },
    )
    .await
    .unwrap()
    .unwrap_or_else(|_| panic!("the tenant should be accepted"));
    (user, tenant)
}

async fn decision(ctx: &AppContext, user_id: String, tenant_id: String) -> Decision {
    let response = AuthorizationService { ctx: ctx.clone() }
        .authorize(Request::new(AuthorizeRequest {
            user_id,
            tenant_id,
            permission_keys: vec!["billing.subscription.read".to_string()],
        }))
        .await
        .unwrap()
        .into_inner();
    Decision::try_from(response.decision).unwrap()
}

#[tokio::test]
#[serial]
async fn an_owner_is_allowed() {
    let boot = boot_test::<App>().await.unwrap();
    let ctx = &boot.app_context;
    let (user, tenant) = owner_and_tenant(ctx).await;
    assert_eq!(
        decision(ctx, user.api_id(), tenant.api_id()).await,
        Decision::Allowed
    );
}

#[tokio::test]
#[serial]
async fn someone_outside_the_tenant_is_denied() {
    let boot = boot_test::<App>().await.unwrap();
    let ctx = &boot.app_context;
    let (_, tenant) = owner_and_tenant(ctx).await;
    let stranger = format!("usr-{}", Uuid::now_v7());
    assert_eq!(
        decision(ctx, stranger, tenant.api_id()).await,
        Decision::Denied
    );
}

#[tokio::test]
#[serial]
async fn a_deactivated_member_is_told_so() {
    let boot = boot_test::<App>().await.unwrap();
    let ctx = &boot.app_context;
    let (user, tenant) = owner_and_tenant(ctx).await;
    let mut membership = memberships::Model::find_in_tenant(&ctx.db, user.id, tenant.id)
        .await
        .unwrap()
        .unwrap()
        .into_active_model();
    membership.deactivated_at = ActiveValue::set(Some(chrono::Utc::now().into()));
    membership.update(&ctx.db).await.unwrap();
    assert_eq!(
        decision(ctx, user.api_id(), tenant.api_id()).await,
        Decision::AccountDeactivated
    );
}

#[tokio::test]
#[serial]
async fn a_plain_uuid_is_invalid_argument() {
    let boot = boot_test::<App>().await.unwrap();
    let status = AuthorizationService {
        ctx: boot.app_context.clone(),
    }
    .authorize(Request::new(AuthorizeRequest {
        user_id: Uuid::now_v7().to_string(),
        tenant_id: Uuid::now_v7().to_string(),
        permission_keys: Vec::new(),
    }))
    .await
    .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}
