use loco_rs::testing::prelude::*;
use serial_test::serial;
use tonic::{Code, Request};
use uuid::Uuid;
use zerpto_iam::{app::App, grpc::seats::SeatsService};
use zerpto_proto::iam::v1::{seats_server::Seats, GetSeatsInUseRequest};

#[tokio::test]
#[serial]
async fn unknown_tenant_has_no_seats() {
    let boot = boot_test::<App>().await.unwrap();
    let service = SeatsService {
        db: boot.app_context.db.clone(),
    };
    let response = service
        .get_seats_in_use(Request::new(GetSeatsInUseRequest {
            tenant_id: Uuid::now_v7().to_string(),
        }))
        .await
        .unwrap();
    assert_eq!(response.into_inner().seats_in_use, 0);
}

#[tokio::test]
#[serial]
async fn malformed_tenant_id_is_invalid_argument() {
    let boot = boot_test::<App>().await.unwrap();
    let service = SeatsService {
        db: boot.app_context.db.clone(),
    };
    let status = service
        .get_seats_in_use(Request::new(GetSeatsInUseRequest {
            tenant_id: "ten-nope".to_string(),
        }))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}
