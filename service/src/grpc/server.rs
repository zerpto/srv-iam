use loco_rs::{app::AppContext, Result};
use tonic::service::Routes;
use zerpto_proto::iam::v1::{
    authorization_server::AuthorizationServer,
    phone_verifications_server::PhoneVerificationsServer, seats_server::SeatsServer,
};

use super::{
    authorization::AuthorizationService, phone_verifications::PhoneVerificationsService,
    seats::SeatsService,
};

/// The gRPC services this service serves. Add each one this service owns with
/// another `.add_service(...)`. The health service is not here: core's
/// `GrpcServer` initializer adds it and starts the server.
///
/// A service that extends this one calls it and adds its own.
///
/// # Errors
///
/// None today; the signature lets a service fail the boot.
#[allow(clippy::unnecessary_wraps)]
pub fn routes(ctx: &AppContext) -> Result<Routes> {
    Ok(Routes::new(AuthorizationServer::new(AuthorizationService {
        ctx: ctx.clone(),
    }))
    .add_service(SeatsServer::new(SeatsService { db: ctx.db.clone() }))
    .add_service(PhoneVerificationsServer::new(PhoneVerificationsService {
        db: ctx.db.clone(),
    })))
}
