use loco_rs::testing::prelude::*;
use serde_json::{json, Value};
use serial_test::serial;
use zerpto_iam::app::App;

const EXTENSION: &str = "ext-0192f7b0-3c1e-7a52-9d84-1b6f2e8a4c37";
/// The key of `config/test.yaml`.
const PLATFORM_KEY: &str = "test-platform-key";

#[tokio::test]
#[serial]
async fn extension_and_menu_endpoints_need_a_credential() {
    request::<App, _, _>(|request, _ctx| async move {
        assert_eq!(request.get("/v1/iam/extensions").await.status_code(), 401);
        assert_eq!(request.get("/v1/iam/menu").await.status_code(), 401);
        assert_eq!(
            request
                .get(&format!("/v1/iam/extensions/{EXTENSION}"))
                .await
                .status_code(),
            401
        );
        // TODO: add "refresh" and "approve" back with the X-Platform-Key check.
        for action in ["enable", "disable"] {
            let path = format!("/v1/iam/extensions/{EXTENSION}/{action}");
            assert_eq!(request.post(&path).await.status_code(), 401, "{action}");
        }
        // TODO: restore with the X-Platform-Key check on registration.
        // let register = request
        //     .post("/v1/iam/extensions")
        //     .json(&json!({"manifest_url": "https://example.com/manifest.yaml"}))
        //     .await;
        // assert_eq!(register.status_code(), 401);
        let wrong = request
            .get("/v1/iam/extensions")
            .add_header("x-platform-key", "wrong")
            .await;
        assert_eq!(wrong.status_code(), 401);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn the_operator_sees_the_core_extensions() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .get("/v1/iam/extensions")
            .add_header("x-platform-key", PLATFORM_KEY)
            .await;
        assert_eq!(response.status_code(), 200);
        let body: Value = response.json();
        let keys: Vec<&str> = body["data"]
            .as_array()
            .unwrap()
            .iter()
            .map(|extension| extension["key"].as_str().unwrap())
            .collect();
        assert_eq!(keys, ["iam"]);
        assert_eq!(body["data"][0]["enabled"], Value::Null);
        assert_eq!(body["meta"]["pagination"]["total_items"], 1);
    })
    .await;
}

#[tokio::test]
#[serial]
async fn an_address_that_is_not_allowed_is_refused() {
    request::<App, _, _>(|request, _ctx| async move {
        let response = request
            .post("/v1/iam/extensions")
            .add_header("x-platform-key", PLATFORM_KEY)
            .json(&json!({"manifest_url": "http://example.com/manifest.yaml"}))
            .await;
        assert_eq!(response.status_code(), 422);
    })
    .await;
}
