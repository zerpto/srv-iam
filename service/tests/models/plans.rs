use loco_rs::testing::prelude::*;
use serial_test::serial;
use uuid::Uuid;
use zerpto_iam::{app::App, plans::provider::provider};

/// The community edition registers a provider that locks nothing.
#[tokio::test]
#[serial]
async fn community_plan_is_unlimited() {
    let boot = boot_test::<App>().await.unwrap();
    let ctx = &boot.app_context;
    let plans = provider(ctx).expect("a plan provider is registered");

    let plan = plans.plan(ctx, Uuid::now_v7()).await.expect("plan");
    assert!(!plan.locked);
    assert_eq!(plan.seat_limit, i64::MAX);
    assert!(plans.features(ctx).await.expect("features").is_empty());
    assert!(!plans
        .trial_available(ctx, Uuid::now_v7())
        .await
        .expect("trial"));
}
