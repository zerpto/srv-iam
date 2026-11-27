//! The reference-data service's region check (`zerpto.referencedata.v1`).

use std::sync::OnceLock;

use loco_rs::app::AppContext;
use tonic::transport::Channel;
use zerpto_core::{helpers, http::problem::Problem};
use zerpto_proto::referencedata::v1::{
    regions_client::RegionsClient, Outcome, ResolveAddressRequest,
};

use crate::{
    models::address::{Address, AddressError, AddressInput, Resolved},
    settings::Settings,
};

static CHANNEL: OnceLock<Channel> = OnceLock::new();

fn unavailable() -> Problem {
    Problem::service_unavailable(
        "reference_data_unavailable",
        "The address could not be checked. Try again shortly.",
    )
}

fn channel(ctx: &AppContext) -> Result<Channel, Problem> {
    if let Some(channel) = CHANNEL.get() {
        return Ok(channel.clone());
    }
    let url = Settings::load(ctx)?.reference_data_url;
    if url.trim().is_empty() {
        tracing::error!("REFERENCE_DATA_GRPC_URL is not set");
        return Err(unavailable());
    }
    let channel = helpers::channel(&url).map_err(|err| {
        tracing::error!(error = %err, "invalid reference-data gRPC url");
        unavailable()
    })?;
    Ok(CHANNEL.get_or_init(|| channel).clone())
}

/// Asks reference-data to check `input` against the region tree and returns
/// the address with the region fields filled in.
///
/// Fails closed: when reference-data cannot answer (not configured, down, a
/// gRPC error) the request is refused with `503 reference_data_unavailable`.
/// A bad address is `Ok(Err(..))`, a normal answer and not an error.
///
/// # Errors
///
/// `503 reference_data_unavailable`, or `500` when the server breaks the contract.
pub async fn resolve(
    ctx: &AppContext,
    input: AddressInput,
) -> Result<Result<Address, AddressError>, Problem> {
    let mut client = RegionsClient::new(channel(ctx)?);
    let response = client
        .resolve_address(ResolveAddressRequest {
            region_id: input
                .region_id
                .map_or_else(String::new, |id| id.to_string()),
            country: input.country.clone().unwrap_or_default(),
        })
        .await
        .map_err(|status| {
            tracing::error!(%status, "reference-data resolve_address call failed");
            unavailable()
        })?
        .into_inner();

    match Outcome::try_from(response.outcome) {
        Ok(Outcome::Ok) => Ok(Ok(Address::complete(
            input,
            Resolved {
                country: response.country,
                province: response.province,
                district: response.district,
                sub_district: response.sub_district,
                village: response.village,
                postal_code: response.postal_code,
            },
        ))),
        Ok(Outcome::UnknownRegion) => Ok(Err(AddressError::UnknownRegion)),
        Ok(Outcome::RegionRequired) => Ok(Err(AddressError::RegionRequired)),
        Ok(Outcome::RegionTooShallow) => Ok(Err(AddressError::RegionTooShallow)),
        Ok(Outcome::CountryRequired) => Ok(Err(AddressError::CountryRequired)),
        Ok(Outcome::Unspecified) | Err(_) => Err(Problem::from(loco_rs::Error::string(
            "reference-data answered with an unknown outcome",
        ))),
    }
}
