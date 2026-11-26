//! The sample passport the passport tests, and the access tests, start from.

use super::super::*;
use crate::product_group::{CarbonFootprint, ProductGroup, RepairabilityScore};

pub(crate) fn make_passport() -> Passport {
    Passport {
        id: PassportId(uuid::Uuid::nil()),
        batch_id: Some("BATCH-001".to_owned()),
        serial_number: None,
        product_name: "Eco Widget".to_owned(),
        product_group: ProductGroup::Electronics,
        manufacturer: ManufacturerInfo {
            name: "ACME Corp".to_owned(),
            address: "123 Main St, Berlin, DE".to_owned(),
            country: None,
            did_web_url: Some("https://acme.example.com/.well-known/did.json".to_owned()),
            registered_trade_name: None,
            electronic_address: None,
        },
        materials: vec![MaterialEntry {
            name: "Recycled Aluminium".to_owned(),
            weight_kg: 0.5,
            recycled_pct: Some(80.0),
            country_of_origin: Some("DE".to_owned()),
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(2.5)),
        repairability_score: Some(RepairabilityScore::from_scalar(7.5)),
        ..crate::test_support::sample_passport()
    }
}
