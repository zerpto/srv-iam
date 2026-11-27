use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;
use zerpto_iam::app::App;

const ROLE: &str = "rol-0191a8c2-7b3e-7c1d-9a52-4f6e8d2b1c07";
const USER: &str = "mem-0191a8c2-7b3e-7c1d-9a52-4f6e8d2b1c07";

#[tokio::test]
#[serial]
async fn role_and_permission_endpoints_need_a_token() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/v1/iam/roles").await.status_code(), 401);
        let create = request
            .post("/v1/iam/roles")
            .json(&json!({"name": "AP clerk", "permissions": ["ap.read"]}))
            .await;
        assert_eq!(create.status_code(), 401);
        assert_eq!(
            request
                .get(&format!("/v1/iam/roles/{ROLE}"))
                .await
                .status_code(),
            401
        );
        let update = request
            .patch(&format!("/v1/iam/roles/{ROLE}"))
            .json(&json!({"name": "Clerk"}))
            .await;
        assert_eq!(update.status_code(), 401);
        assert_eq!(
            request
                .delete(&format!("/v1/iam/roles/{ROLE}"))
                .await
                .status_code(),
            401
        );
        assert_eq!(request.get("/v1/iam/permissions").await.status_code(), 401);
        assert_eq!(
            request
                .get(&format!("/v1/iam/users/{USER}/permissions"))
                .await
                .status_code(),
            401
        );
        let put = request
            .put(&format!("/v1/iam/users/{USER}/permissions"))
            .json(&json!({"grants": []}))
            .await;
        assert_eq!(put.status_code(), 401);
    })
    .await;
}
