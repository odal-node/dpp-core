//! The in-memory repository double and the passports the repository tests share.

use super::super::port::*;
use async_trait::async_trait;

use crate::error::DppError;
use crate::identifier::ProductIdentifier;
use crate::passport::ManufacturerInfo;
use crate::product_group::ProductGroup;
use crate::{
    passport::{Passport, PassportId},
    status::PassportStatus,
};
use std::collections::HashMap;
use std::sync::Mutex;

/// Minimal in-memory repo to exercise the trait's **default** method bodies
/// (`patch_fields`, `create_batch`, `update_batch`). Only the methods those
/// defaults call are functional; the rest satisfy the signature.
#[derive(Default)]
pub(super) struct InMemoryRepo {
    store: Mutex<HashMap<PassportId, Passport>>,
}

#[async_trait]
impl PassportRepository for InMemoryRepo {
    async fn create(&self, passport: Passport) -> Result<Passport, DppError> {
        self.store
            .lock()
            .unwrap()
            .insert(passport.id, passport.clone());
        Ok(passport)
    }
    async fn find_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError> {
        Ok(self.store.lock().unwrap().get(&id).cloned())
    }
    async fn find_published_by_id(&self, id: PassportId) -> Result<Option<Passport>, DppError> {
        self.find_by_id(id).await
    }
    async fn find_by_id_any_status(&self, id: PassportId) -> Result<Option<Passport>, DppError> {
        self.find_by_id(id).await
    }
    async fn update(&self, passport: Passport) -> Result<Passport, DppError> {
        self.store
            .lock()
            .unwrap()
            .insert(passport.id, passport.clone());
        Ok(passport)
    }
    async fn update_status(
        &self,
        id: PassportId,
        status: PassportStatus,
    ) -> Result<Passport, DppError> {
        let mut g = self.store.lock().unwrap();
        let mut p = g
            .get(&id)
            .cloned()
            .ok_or(DppError::NotFound(id.to_string()))?;
        p.status = status;
        g.insert(id, p.clone());
        Ok(p)
    }
    async fn list(
        &self,
        status: Option<PassportStatus>,
        _q: Option<&str>,
        _facility_id: Option<&str>,
        _limit: u32,
        _offset: u32,
    ) -> Result<Vec<Passport>, DppError> {
        // 🚨 This ignored `status` entirely, which made it a double that does
        // not model the contract it stands in for: every default implementation
        // written in terms of `list(Some(..))` was exercised against a store
        // that answered with everything. `None` is every status, matching the
        // `facility_id` reading on the trait.
        Ok(self
            .store
            .lock()
            .unwrap()
            .values()
            .filter(|p| status.as_ref().is_none_or(|s| p.status == *s))
            .cloned()
            .collect())
    }
    async fn count(
        &self,
        _status: Option<PassportStatus>,
        _facility_id: Option<&str>,
    ) -> Result<u64, DppError> {
        Ok(self.store.lock().unwrap().len() as u64)
    }
}

/// The identifier every fixture below carries — the GTIN on
/// `sample_battery_data`, so a lookup has something real to match.
pub(super) fn fixture_identifier() -> ProductIdentifier {
    ProductIdentifier::gs1(crate::Gtin::parse("09506000134352").expect("a GTIN"))
}

/// A draft passport that carries product group data, and therefore an
/// identifier. `sample_passport` deliberately has none.
pub(super) fn identified_passport(name: &str) -> Passport {
    Passport {
        product_group: ProductGroup::Battery,
        product_group_data: Some(crate::ProductGroupData::Battery(Box::new(
            crate::test_support::sample_battery_data(),
        ))),
        ..draft_passport(name)
    }
}

pub(super) fn draft_passport(name: &str) -> Passport {
    Passport {
        product_name: name.into(),
        manufacturer: ManufacturerInfo {
            name: "Brand".into(),
            address: "Berlin, DE".into(),
            country: None,
            did_web_url: None,
            registered_trade_name: None,
            electronic_address: None,
        },
        schema_version: "1.1.0".into(),
        ..crate::test_support::sample_passport()
    }
}
