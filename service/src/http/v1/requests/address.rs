use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;
use zerpto_core::{helpers::non_blank, http::ids::ApiIds, validation::failure};

use crate::{http::ids::REGION, models::address::AddressInput};

fn check_region_id(value: &str) -> Result<(), validator::ValidationError> {
    if ApiIds::new(REGION).parse(value).is_none() {
        return Err(failure("unknown_region", "This region does not exist."));
    }
    Ok(())
}

/// A postal address (`Address` in the API contract).
///
/// The region is checked by the reference-data service (`grpc::regions`).
#[derive(Debug, Clone, Deserialize, Serialize, Validate, TS)]
pub struct AddressRequest {
    #[validate(custom(function = "check_region_id"))]
    #[serde(default)]
    pub region_id: Option<String>,
    #[serde(default)]
    pub line1: Option<String>,
    #[serde(default)]
    pub line2: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub province: Option<String>,
    #[serde(default)]
    pub district: Option<String>,
    #[serde(default)]
    pub sub_district: Option<String>,
    #[serde(default)]
    pub village: Option<String>,
    #[serde(default)]
    pub postal_code: Option<String>,
}

impl AddressRequest {
    /// Trims the checked body into what the model takes.
    #[must_use]
    pub fn into_domain(self) -> AddressInput {
        AddressInput {
            region_id: self
                .region_id
                .as_deref()
                .and_then(|id| ApiIds::new(REGION).parse(id)),
            line1: non_blank(self.line1),
            line2: non_blank(self.line2),
            country: non_blank(self.country),
            province: non_blank(self.province),
            district: non_blank(self.district),
            sub_district: non_blank(self.sub_district),
            village: non_blank(self.village),
            postal_code: non_blank(self.postal_code),
        }
    }
}
