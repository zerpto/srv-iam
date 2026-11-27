use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;
use zerpto_iam::app::App;

#[tokio::test]
#[serial]
async fn sign_up_rejects_a_short_password() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/v1/auth/sign-up")
            .json(&json!({"name": "Siti Rahma", "email": "siti@example.com", "password": "short"}))
            .await;
        assert_eq!(response.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn sign_up_creates_a_pending_account() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/v1/auth/sign-up")
            .json(&json!({"name": "Siti Rahma", "email": "Siti@Example.com", "password": "a-long-enough-password"}))
            .await;
        assert_eq!(response.status_code(), 201);
        let body: serde_json::Value = response.json();
        assert_eq!(body["data"]["email"], "siti@example.com");
        assert_eq!(body["data"]["status"], "pending_verification");
        assert_eq!(body["meta"]["actions"][0]["href"], "/v1/auth/sign-in");

        let again = request
            .post("/v1/auth/sign-up")
            .json(&json!({"name": "Siti Rahma", "email": "siti@example.com", "password": "a-long-enough-password"}))
            .await;
        assert_eq!(again.status_code(), 409);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn sign_in_is_refused_until_the_email_is_confirmed() {
    request::<App, _, _>(|request, _ctx| async move {
        request
            .post("/v1/auth/sign-up")
            .json(&json!({"name": "Budi Santoso", "email": "budi@example.com", "password": "a-long-enough-password"}))
            .await;
        let response = request
            .post("/v1/auth/sign-in")
            .json(&json!({"email": "budi@example.com", "password": "a-long-enough-password"}))
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn me_needs_a_token() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/v1/auth/me").await.status_code(), 401);
        assert_eq!(request.delete("/v1/auth/sign-out").await.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn refresh_refuses_an_unknown_token() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/v1/auth/refresh")
            .json(&json!({"refresh_token": "nope.nope"}))
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn forgot_password_answers_202_for_any_address() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/v1/auth/reset-password")
            .json(&json!({"email": "nobody@example.com"}))
            .await;
        assert_eq!(response.status_code(), 202);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn reset_password_refuses_an_unknown_token() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .put("/v1/auth/reset-password")
            .json(&json!({"token": "nope", "new_password": "a-long-enough-password"}))
            .await;
        assert_eq!(response.status_code(), 400);

        let weak = request
            .put("/v1/auth/reset-password")
            .json(&json!({"token": "nope", "new_password": "short"}))
            .await;
        assert_eq!(weak.status_code(), 422);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn switch_tenant_needs_a_token() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/v1/auth/switch-tenant")
            .json(&json!({"tenant_id": "ten-0191a8c2-7b3e-7c1d-9a52-4f6e8d2b1c07"}))
            .await;
        assert_eq!(response.status_code(), 401);
    })
    .await;
}
