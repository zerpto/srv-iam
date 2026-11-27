use loco_rs::testing::prelude::*;
use serde_json::json;
use serial_test::serial;
use zerpto_iam::app::App;

#[tokio::test]
#[serial]
async fn tenant_endpoints_need_a_token() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/v1/iam/tenant").await.status_code(), 401);
        assert_eq!(
            request
                .patch("/v1/iam/tenant")
                .json(&json!({"name": "Acme"}))
                .await
                .status_code(),
            401
        );
        assert_eq!(request.get("/v1/iam/tenants").await.status_code(), 401);
        assert_eq!(
            request
                .post("/v1/iam/tenants")
                .json(&json!({"name": "Acme"}))
                .await
                .status_code(),
            401
        );
    })
    .await;
}
