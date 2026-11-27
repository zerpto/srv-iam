use loco_rs::testing::prelude::*;
use serial_test::serial;
use zerpto_iam::app::App;

#[tokio::test]
#[serial]
async fn health_returns_the_envelope() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request.get("/v1/health").await;
        assert_eq!(response.status_code(), 200);
        let body: serde_json::Value = response.json();
        assert_eq!(body["data"]["status"], "ok");
        assert!(body["meta"].is_object());
    })
    .await;
}
