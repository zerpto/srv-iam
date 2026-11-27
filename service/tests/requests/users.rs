use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;
use zerpto_iam::app::App;

const ID: &str = "mem-0191a8c2-7b3e-7c1d-9a52-4f6e8d2b1c07";

#[tokio::test]
#[serial]
async fn user_endpoints_need_a_token() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/v1/iam/users").await.status_code(), 401);
        let invite = request
            .post("/v1/iam/users")
            .json(&json!({"email": "a@example.com", "role_ids": []}))
            .await;
        assert_eq!(invite.status_code(), 401);
        assert_eq!(
            request
                .get(&format!("/v1/iam/users/{ID}"))
                .await
                .status_code(),
            401
        );
        let update = request
            .patch(&format!("/v1/iam/users/{ID}"))
            .json(&json!({"role_ids": []}))
            .await;
        assert_eq!(update.status_code(), 401);
        assert_eq!(
            request
                .delete(&format!("/v1/iam/users/{ID}"))
                .await
                .status_code(),
            401
        );
        assert_eq!(
            request
                .put(&format!("/v1/iam/users/{ID}/activate"))
                .await
                .status_code(),
            401
        );
        assert_eq!(
            request
                .delete(&format!("/v1/iam/users/{ID}/activate"))
                .await
                .status_code(),
            401
        );
    })
    .await;
}
