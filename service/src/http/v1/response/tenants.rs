use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::{address::Address, auth::TokenPair};
use crate::models::{
    memberships,
    scope::Scope,
    tenants::{self, COMPLEX, DEFAULT_CURRENCY, DEFAULT_FISCAL_YEAR_START_MONTH, DEFAULT_TIMEZONE},
};

/// The profile and settings of a tenant (`Tenant` in the API contract).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Tenant {
    pub id: String,
    pub name: String,
    pub legal_name: Option<String>,
    /// NPWP.
    pub tax_id: Option<String>,
    /// Left out until an address has been set.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub address: Option<Address>,
    pub base_currency: String,
    pub timezone: String,
    #[ts(type = "number")]
    pub fiscal_year_start_month: i64,
    /// `simple` or `complex`.
    pub completeness: String,
    pub created_at: DateTime<Utc>,
}

impl From<tenants::Model> for Tenant {
    fn from(tenant: tenants::Model) -> Self {
        Self {
            id: tenant.api_id(),
            address: tenant.address().map(Address::from),
            name: tenant.name,
            legal_name: tenant.legal_name,
            tax_id: tenant.tax_id,
            base_currency: tenant
                .base_currency
                .unwrap_or_else(|| DEFAULT_CURRENCY.to_string()),
            timezone: tenant
                .timezone
                .unwrap_or_else(|| DEFAULT_TIMEZONE.to_string()),
            fiscal_year_start_month: tenant
                .fiscal_year_start_month
                .unwrap_or(DEFAULT_FISCAL_YEAR_START_MONTH),
            completeness: tenant.completeness.unwrap_or_else(|| COMPLEX.to_string()),
            created_at: tenant.created_at.with_timezone(&Utc),
        }
    }
}

/// One tenant the signed-in person belongs to (`Membership`).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct Membership {
    pub id: String,
    pub tenant: TenantSummary,
    /// True for the tenant the current token is scoped to.
    pub is_active: bool,
}

impl Membership {
    #[must_use]
    pub fn new(membership: &memberships::Model, tenant: TenantSummary, is_active: bool) -> Self {
        Self {
            id: membership.api_id(),
            tenant,
            is_active,
        }
    }
}

/// The tenant a session is scoped to, with the user's role names in it.
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
pub struct TenantSummary {
    pub id: String,
    pub name: String,
    pub roles: Vec<String>,
}

impl From<&Scope> for TenantSummary {
    fn from(scope: &Scope) -> Self {
        Self {
            id: scope.tenant.api_id(),
            name: scope.tenant.name.clone(),
            roles: scope.roles.iter().map(|role| role.name.clone()).collect(),
        }
    }
}

/// A new token pair scoped to a tenant, with a summary of that tenant
/// (`TenantSession`).
#[derive(Debug, Serialize, Deserialize, TS)]
pub struct TenantSession {
    #[serde(flatten)]
    pub tokens: TokenPair,
    pub tenant: TenantSummary,
}
