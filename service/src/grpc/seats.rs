//! Seat usage of a tenant (`zerpto.iam.v1.Seats`), asked by billing.

use sea_orm::DatabaseConnection;
use tonic::{Request, Response, Status};
use zerpto_proto::iam::v1::{seats_server::Seats, GetSeatsInUseRequest, GetSeatsInUseResponse};

use super::parse_id::parse_id;
use crate::models::members;

/// Answers `Seats` from this service's database.
pub struct SeatsService {
    pub db: DatabaseConnection,
}

#[tonic::async_trait]
impl Seats for SeatsService {
    /// Active members plus pending invitations. An unknown tenant has none.
    async fn get_seats_in_use(
        &self,
        request: Request<GetSeatsInUseRequest>,
    ) -> Result<Response<GetSeatsInUseResponse>, Status> {
        let tenant_id = parse_id("tenant_id", &request.get_ref().tenant_id)?;
        let seats_in_use = members::seats_in_use(&self.db, tenant_id, None)
            .await
            .map_err(|err| {
                tracing::error!(error = %err, "seats_in_use query failed");
                Status::internal("could not count seats")
            })?;
        Ok(Response::new(GetSeatsInUseResponse { seats_in_use }))
    }
}
