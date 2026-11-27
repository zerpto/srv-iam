//! Whether a user has a verified phone (`zerpto.iam.v1.PhoneVerifications`),
//! asked by billing before it starts a trial.

use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};
use zerpto_proto::iam::v1::{
    phone_verifications_server::PhoneVerifications, GetPhoneVerificationRequest,
    GetPhoneVerificationResponse,
};

use super::parse_id::parse_id;
use crate::models::users;

/// Answers `PhoneVerifications` from this service's database.
pub struct PhoneVerificationsService {
    pub db: DatabaseConnection,
}

#[tonic::async_trait]
impl PhoneVerifications for PhoneVerificationsService {
    /// The hash is the stored one (hex), never the number; empty when none.
    /// An unknown user is not verified.
    async fn get_phone_verification(
        &self,
        request: Request<GetPhoneVerificationRequest>,
    ) -> Result<Response<GetPhoneVerificationResponse>, Status> {
        let user_id = parse_id("user_id", &request.get_ref().user_id)?;
        let user = users::Model::find_by_id(&self.db, user_id)
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "phone verification lookup failed");
                Status::internal("could not read the phone verification")
            })?;
        let response = user.map_or_else(
            || GetPhoneVerificationResponse {
                verified: false,
                phone_hash: String::new(),
            },
            |user| GetPhoneVerificationResponse {
                verified: user.is_phone_verified(),
                phone_hash: user.phone_hash.unwrap_or_default(),
            },
        );
        Ok(Response::new(response))
    }
}
