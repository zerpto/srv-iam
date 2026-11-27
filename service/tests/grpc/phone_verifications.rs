use loco_rs::testing::prelude::*;
use serial_test::serial;
use tonic::{Code, Request};
use uuid::Uuid;
use zerpto_iam::{app::App, grpc::phone_verifications::PhoneVerificationsService};
use zerpto_proto::iam::v1::{
    phone_verifications_server::PhoneVerifications, GetPhoneVerificationRequest,
};

#[tokio::test]
#[serial]
async fn unknown_user_is_not_verified() {
    let boot = boot_test::<App>().await.unwrap();
    let service = PhoneVerificationsService {
        db: boot.app_context.db.clone(),
    };
    let response = service
        .get_phone_verification(Request::new(GetPhoneVerificationRequest {
            user_id: Uuid::now_v7().to_string(),
        }))
        .await
        .unwrap()
        .into_inner();
    assert!(!response.verified);
    assert!(response.phone_hash.is_empty());
}

#[tokio::test]
#[serial]
async fn malformed_user_id_is_invalid_argument() {
    let boot = boot_test::<App>().await.unwrap();
    let service = PhoneVerificationsService {
        db: boot.app_context.db.clone(),
    };
    let status = service
        .get_phone_verification(Request::new(GetPhoneVerificationRequest {
            user_id: "usr-nope".to_string(),
        }))
        .await
        .unwrap_err();
    assert_eq!(status.code(), Code::InvalidArgument);
}
