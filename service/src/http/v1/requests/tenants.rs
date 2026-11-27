use serde::{Deserialize, Serialize};
use ts_rs::TS;
use validator::Validate;
use zerpto_core::{helpers::non_blank, http::nullable::Nullable, validation::failure};

use super::address::AddressRequest;
use crate::models::{
    address::{Address, AddressInput},
    tenants::{
        NewTenant, TenantChanges, COMPLEX, DEFAULT_CURRENCY, DEFAULT_FISCAL_YEAR_START_MONTH,
        DEFAULT_TIMEZONE, SIMPLE,
    },
};

fn check_timezone(value: &str) -> Result<(), validator::ValidationError> {
    if value.trim().parse::<chrono_tz::Tz>().is_err() {
        return Err(failure(
            "invalid_timezone",
            "Timezone must be an IANA name such as Asia/Jakarta.",
        ));
    }
    Ok(())
}

fn check_currency(value: &str) -> Result<(), validator::ValidationError> {
    let code = value.trim();
    if code.len() != 3 || !code.chars().all(|c| c.is_ascii_alphabetic()) {
        return Err(failure(
            "invalid_base_currency",
            "Currency must be a three-letter ISO 4217 code.",
        ));
    }
    Ok(())
}

fn nullable_tax_id(value: &Nullable<String>) -> Result<(), validator::ValidationError> {
    match value {
        Nullable::Value(tax_id) => zerpto_core::validation::rules::tax_id(tax_id),
        Nullable::Missing | Nullable::Null => Ok(()),
    }
}

fn check_completeness(value: &str) -> Result<(), validator::ValidationError> {
    if value != SIMPLE && value != COMPLEX {
        return Err(failure(
            "invalid_completeness",
            "`completeness` must be `simple` or `complex`.",
        ));
    }
    Ok(())
}

/// `POST /v1/iam/tenants` (`TenantInput` in the API contract).
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct CreateTenantRequest {
    #[validate(custom(function = "zerpto_core::validation::rules::name"))]
    pub name: String,
    #[serde(default)]
    pub legal_name: Option<String>,
    /// NPWP.
    #[validate(custom(function = "zerpto_core::validation::rules::tax_id"))]
    #[serde(default)]
    pub tax_id: Option<String>,
    #[validate(nested)]
    #[serde(default)]
    pub address: Option<AddressRequest>,
    #[validate(custom(function = "check_currency"))]
    #[serde(default)]
    pub base_currency: Option<String>,
    #[validate(custom(function = "check_timezone"))]
    #[serde(default)]
    pub timezone: Option<String>,
    #[validate(range(
        min = 1,
        max = 12,
        code = "invalid_fiscal_year_start_month",
        message = "Fiscal year start month must be between 1 and 12."
    ))]
    #[ts(type = "number | null")]
    #[serde(default)]
    pub fiscal_year_start_month: Option<i64>,
    #[validate(custom(function = "check_completeness"))]
    #[serde(default)]
    pub completeness: Option<String>,
}

impl CreateTenantRequest {
    /// The address as typed, to be checked by reference-data before it is saved.
    #[must_use]
    pub fn address_input(&self) -> Option<AddressInput> {
        self.address.clone().map(AddressRequest::into_domain)
    }

    /// Applies the contract's defaults to the checked body. `address` is the
    /// one reference-data resolved from [`address_input`](Self::address_input).
    #[must_use]
    pub fn into_domain(self, address: Option<Address>) -> NewTenant {
        NewTenant {
            name: self.name.trim().to_string(),
            legal_name: non_blank(self.legal_name),
            tax_id: non_blank(self.tax_id),
            base_currency: non_blank(self.base_currency)
                .map_or_else(|| DEFAULT_CURRENCY.to_string(), |c| c.to_uppercase()),
            timezone: non_blank(self.timezone).unwrap_or_else(|| DEFAULT_TIMEZONE.to_string()),
            fiscal_year_start_month: self
                .fiscal_year_start_month
                .unwrap_or(DEFAULT_FISCAL_YEAR_START_MONTH),
            completeness: self.completeness.unwrap_or_else(|| COMPLEX.to_string()),
            address,
        }
    }
}

/// `PATCH /v1/iam/tenant` (`TenantUpdate`): every field is optional and only
/// those sent change. The base currency is not accepted.
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct UpdateTenantRequest {
    #[validate(custom(function = "zerpto_core::validation::rules::name"))]
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub legal_name: Option<String>,
    /// NPWP. `null` clears it.
    #[validate(custom(function = "nullable_tax_id"))]
    #[serde(default)]
    #[ts(optional, type = "string | null")]
    pub tax_id: Nullable<String>,
    #[validate(nested)]
    #[serde(default)]
    pub address: Option<AddressRequest>,
    #[validate(custom(function = "check_timezone"))]
    #[serde(default)]
    pub timezone: Option<String>,
    #[validate(range(
        min = 1,
        max = 12,
        code = "invalid_fiscal_year_start_month",
        message = "Fiscal year start month must be between 1 and 12."
    ))]
    #[ts(type = "number | null")]
    #[serde(default)]
    pub fiscal_year_start_month: Option<i64>,
    #[validate(custom(function = "check_completeness"))]
    #[serde(default)]
    pub completeness: Option<String>,
}

impl UpdateTenantRequest {
    /// The address as typed, to be checked by reference-data before it is saved.
    #[must_use]
    pub fn address_input(&self) -> Option<AddressInput> {
        self.address.clone().map(AddressRequest::into_domain)
    }

    /// Trims the checked body into what the model takes. `address` is the one
    /// reference-data resolved from [`address_input`](Self::address_input).
    #[must_use]
    pub fn into_domain(self, address: Option<Address>) -> TenantChanges {
        TenantChanges {
            name: self.name.map(|name| name.trim().to_string()),
            legal_name: self.legal_name.map(|v| non_blank(Some(v))),
            tax_id: self.tax_id.into_change().map(non_blank),
            address,
            timezone: self.timezone.map(|zone| zone.trim().to_string()),
            fiscal_year_start_month: self.fiscal_year_start_month,
            completeness: self.completeness,
        }
    }
}

/// `POST /v1/auth/switch-tenant`
#[derive(Debug, Deserialize, Serialize, Validate, TS)]
pub struct SwitchTenantRequest {
    #[validate(length(min = 1))]
    pub tenant_id: String,
}
