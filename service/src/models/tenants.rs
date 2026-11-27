pub use super::_entities::tenants::{ActiveModel, Column, Entity, Model};
use loco_rs::prelude::*;
use uuid::Uuid;
use zerpto_core::http::ids::ApiIds;

use super::{address::Address, memberships, roles, tenant_extensions};
use crate::http::ids::TENANT;

pub const DEFAULT_CURRENCY: &str = "IDR";
pub const DEFAULT_TIMEZONE: &str = "Asia/Jakarta";
pub const DEFAULT_FISCAL_YEAR_START_MONTH: i64 = 1;

/// How much of the product a tenant uses. The web app shows modules by it; the
/// API does not restrict anything.
pub const SIMPLE: &str = "simple";
pub const COMPLEX: &str = "complex";

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    async fn before_save<C>(mut self, _db: &C, insert: bool) -> Result<Self, DbErr>
    where
        C: ConnectionTrait,
    {
        if insert && self.id.is_not_set() {
            self.id = ActiveValue::Set(Uuid::now_v7());
        }
        if !insert && self.updated_at.is_unchanged() {
            self.updated_at = ActiveValue::Set(chrono::Utc::now().into());
        }
        Ok(self)
    }
}

/// Why a tenant's details were refused.
#[derive(Debug, PartialEq, Eq)]
pub enum InputError {
    TaxIdTaken,
}

/// A change to a tenant's details, already checked for shape.
///
/// The address is already resolved by reference-data. Every field is optional: only
/// those present are changed, and a present `None` clears the value.
#[derive(Debug, Default)]
pub struct TenantChanges {
    pub name: Option<String>,
    pub legal_name: Option<Option<String>>,
    pub tax_id: Option<Option<String>>,
    pub address: Option<Address>,
    pub timezone: Option<String>,
    pub fiscal_year_start_month: Option<i64>,
    pub completeness: Option<String>,
}

/// What a person types to create a tenant, already checked for shape.
#[derive(Debug)]
pub struct NewTenant {
    pub name: String,
    pub legal_name: Option<String>,
    pub tax_id: Option<String>,
    pub base_currency: String,
    pub timezone: String,
    pub fiscal_year_start_month: i64,
    pub completeness: String,
    pub address: Option<Address>,
}

impl ActiveModel {
    /// Replaces the whole address with a resolved one.
    fn set_address(&mut self, address: Address) {
        self.region_id = ActiveValue::set(address.region_id);
        self.line1 = ActiveValue::set(address.line1);
        self.line2 = ActiveValue::set(address.line2);
        self.country = ActiveValue::set(address.country);
        self.province = ActiveValue::set(address.province);
        self.district = ActiveValue::set(address.district);
        self.sub_district = ActiveValue::set(address.sub_district);
        self.village = ActiveValue::set(address.village);
        self.postal_code = ActiveValue::set(address.postal_code);
    }
}

impl Model {
    /// The API id: `ten-<uuid>`.
    #[must_use]
    pub fn api_id(&self) -> String {
        ApiIds::new(TENANT).encode(&self.id)
    }

    /// # Errors
    ///
    /// When the query fails or the tenant does not exist.
    pub async fn find_by_id<C: ConnectionTrait>(db: &C, id: Uuid) -> ModelResult<Self> {
        Entity::find_by_id(id)
            .one(db)
            .await?
            .ok_or(ModelError::EntityNotFound)
    }

    /// The address, if one has been set.
    #[must_use]
    pub fn address(&self) -> Option<Address> {
        self.country.as_ref()?;
        Some(Address {
            region_id: self.region_id,
            line1: self.line1.clone(),
            line2: self.line2.clone(),
            country: self.country.clone(),
            province: self.province.clone(),
            district: self.district.clone(),
            sub_district: self.sub_district.clone(),
            village: self.village.clone(),
            postal_code: self.postal_code.clone(),
        })
    }

    /// Creates a tenant and makes `owner_id` its owner, in one transaction.
    ///
    /// The monolith also seeded the default chart of accounts here; that is
    /// the accounting service's. TODO: publish a tenant-created event through
    /// the outbox, in this transaction, so it can seed itself.
    ///
    /// # Errors
    ///
    /// `Ok(Err(..))` says why the input was refused; the outer error is a
    /// failed query.
    pub async fn create_with_owner(
        db: &DatabaseConnection,
        owner_id: Uuid,
        new: NewTenant,
    ) -> ModelResult<Result<Self, InputError>> {
        let txn = db.begin().await?;

        if let Some(tax_id) = &new.tax_id {
            if Self::tax_id_taken(&txn, tax_id, None).await? {
                return Ok(Err(InputError::TaxIdTaken));
            }
        }
        let mut tenant = ActiveModel {
            name: ActiveValue::set(new.name),
            legal_name: ActiveValue::set(new.legal_name),
            tax_id: ActiveValue::set(new.tax_id),
            base_currency: ActiveValue::set(Some(new.base_currency)),
            timezone: ActiveValue::set(Some(new.timezone)),
            fiscal_year_start_month: ActiveValue::set(Some(new.fiscal_year_start_month)),
            completeness: ActiveValue::set(Some(new.completeness)),
            ..Default::default()
        };
        if let Some(address) = new.address {
            tenant.set_address(address);
        }
        let tenant = tenant.insert(&txn).await?;

        let owner_role = roles::Model::find_system(&txn, roles::OWNER)
            .await?
            .ok_or_else(|| ModelError::msg("the system `owner` role is missing"))?;
        memberships::Model::create_with_roles(
            &txn,
            Uuid::now_v7(),
            owner_id,
            tenant.id,
            &[owner_role.id],
        )
        .await?;
        tenant_extensions::Model::enable_all_approved(&txn, tenant.id).await?;

        txn.commit().await?;
        Ok(Ok(tenant))
    }

    /// Whether another tenant already uses this tax id (NPWP).
    async fn tax_id_taken<C: ConnectionTrait>(
        db: &C,
        tax_id: &str,
        except: Option<Uuid>,
    ) -> ModelResult<bool> {
        let mut query = Entity::find().filter(Column::TaxId.eq(tax_id));
        if let Some(id) = except {
            query = query.filter(Column::Id.ne(id));
        }
        Ok(query.one(db).await?.is_some())
    }

    /// Applies `changes`. A new address replaces the whole old one.
    ///
    /// # Errors
    ///
    /// `Ok(Err(..))` says why the input was refused; the outer error is a
    /// failed query.
    pub async fn apply(
        self,
        db: &DatabaseConnection,
        changes: TenantChanges,
    ) -> ModelResult<Result<Self, InputError>> {
        let txn = db.begin().await?;
        let mut tenant = self.clone().into_active_model();

        if let Some(Some(tax_id)) = &changes.tax_id {
            if Self::tax_id_taken(&txn, tax_id, Some(self.id)).await? {
                return Ok(Err(InputError::TaxIdTaken));
            }
        }
        if let Some(address) = changes.address {
            tenant.set_address(address);
        }
        if let Some(name) = changes.name {
            tenant.name = ActiveValue::set(name);
        }
        if let Some(legal_name) = changes.legal_name {
            tenant.legal_name = ActiveValue::set(legal_name);
        }
        if let Some(tax_id) = changes.tax_id {
            tenant.tax_id = ActiveValue::set(tax_id);
        }
        if let Some(timezone) = changes.timezone {
            tenant.timezone = ActiveValue::set(Some(timezone));
        }
        if let Some(month) = changes.fiscal_year_start_month {
            tenant.fiscal_year_start_month = ActiveValue::set(Some(month));
        }
        if let Some(completeness) = changes.completeness {
            tenant.completeness = ActiveValue::set(Some(completeness));
        }
        let tenant = tenant.update(&txn).await?;
        txn.commit().await?;
        Ok(Ok(tenant))
    }
}
