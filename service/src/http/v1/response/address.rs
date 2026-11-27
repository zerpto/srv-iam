use serde::{Deserialize, Serialize};
use ts_rs::TS;
use zerpto_core::http::ids::ApiIds;

use crate::http::ids::REGION;

use crate::models::address;

/// A postal address (`Address` in the API contract).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Address {
    pub region_id: Option<String>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub country: Option<String>,
    pub province: Option<String>,
    pub district: Option<String>,
    pub sub_district: Option<String>,
    pub village: Option<String>,
    pub postal_code: Option<String>,
}

impl From<address::Address> for Address {
    fn from(address: address::Address) -> Self {
        Self {
            region_id: address.region_id.map(|id| ApiIds::new(REGION).encode(&id)),
            line1: address.line1,
            line2: address.line2,
            country: address.country,
            province: address.province,
            district: address.district,
            sub_district: address.sub_district,
            village: address.village,
            postal_code: address.postal_code,
        }
    }
}
