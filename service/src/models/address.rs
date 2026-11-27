//! A postal address as a tenant stores it.
//!
//! Regions belong to the reference-data service. It checks an address against
//! the region tree (`grpc::regions::resolve`) and answers with the region
//! fields; [`Address::complete`] merges that answer with what the person typed.

use uuid::Uuid;

/// Why an address was refused. Each is an outcome of the reference-data check.
#[derive(Debug, PartialEq, Eq)]
pub enum AddressError {
    UnknownRegion,
    RegionRequired,
    RegionTooShallow,
    CountryRequired,
}

/// An address as the client sends it, already checked for shape.
#[derive(Debug, Default, Clone)]
pub struct AddressInput {
    pub region_id: Option<Uuid>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub country: Option<String>,
    pub province: Option<String>,
    pub district: Option<String>,
    pub sub_district: Option<String>,
    pub village: Option<String>,
    pub postal_code: Option<String>,
}

/// What reference-data filled in from the region tree. An empty string means
/// the tree has nothing for that level.
#[derive(Debug, Default, Clone)]
pub struct Resolved {
    pub country: String,
    pub province: String,
    pub district: String,
    pub sub_district: String,
    pub village: String,
    pub postal_code: String,
}

/// A stored address.
#[derive(Debug, Default, Clone)]
pub struct Address {
    pub region_id: Option<Uuid>,
    pub line1: Option<String>,
    pub line2: Option<String>,
    pub country: Option<String>,
    pub province: Option<String>,
    pub district: Option<String>,
    pub sub_district: Option<String>,
    pub village: Option<String>,
    pub postal_code: Option<String>,
}

fn pick(resolved: String, typed: Option<String>) -> Option<String> {
    if resolved.is_empty() {
        typed
    } else {
        Some(resolved)
    }
}

impl Address {
    /// Merges the region fields reference-data returned into the typed
    /// address. What the tree gives replaces what was typed for that level;
    /// levels below the tree (the village when it stops at the sub-district)
    /// and a postal code no region supplies stay as typed.
    #[must_use]
    pub fn complete(input: AddressInput, resolved: Resolved) -> Self {
        let country = if resolved.country.is_empty() {
            input.country.map(|c| c.trim().to_uppercase())
        } else {
            Some(resolved.country)
        };
        Self {
            region_id: input.region_id,
            line1: input.line1,
            line2: input.line2,
            country,
            province: pick(resolved.province, input.province),
            district: pick(resolved.district, input.district),
            sub_district: pick(resolved.sub_district, input.sub_district),
            village: pick(resolved.village, input.village),
            postal_code: pick(resolved.postal_code, input.postal_code),
        }
    }
}
