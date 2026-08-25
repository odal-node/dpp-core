//! What `Passport::validate` accepts and what it refuses, field by field.

use super::*;
use crate::domain::product_group::{
    BatteryChemistry, BatteryData, CarbonFootprint, ProductGroup, ProductGroupData,
    RepairabilityScore,
};
use crate::domain::status::PassportStatus;
use chrono::Utc;
use uuid::Uuid;

use super::tests::make_passport;

#[test]
fn validate_valid_passport_ok() {
    let p = make_passport();
    assert!(p.validate().is_ok());
}

#[test]
fn validate_empty_product_name() {
    let mut p = make_passport();
    p.product_name = "".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("product_name"), "got: {err}");
}

/// The placing-on-market date has two homes and they may not contradict.
///
/// It selects which law binds the product, so two answers is two different sets
/// of obligations and nothing downstream can tell which was meant. The envelope
/// field is the one a determination reads; battery's own predates it and is in
/// released schemas.
#[test]
fn validate_rejects_two_disagreeing_placing_on_market_dates() {
    let day = |d: u32| chrono::NaiveDate::from_ymd_opt(2031, 8, d).expect("valid date");
    let mut p = make_passport();
    p.product_group = ProductGroup::Battery;
    p.product_group_data = Some(ProductGroupData::Battery(Box::new(BatteryData {
        placed_on_market_date: Some(day(18)),
        ..crate::test_support::sample_battery_data()
    })));

    p.placed_on_market_date = Some(day(17));
    let err = p.validate().unwrap_err().to_string();
    assert!(
        err.contains("placed_on_market_date") && err.contains("two values"),
        "got: {err}"
    );

    // Agreeing is fine, and so is either one being absent — the envelope field
    // is optional and a passport that declares neither has simply not said.
    p.placed_on_market_date = Some(day(18));
    assert!(p.validate().is_ok(), "{:?}", p.validate());
    p.placed_on_market_date = None;
    assert!(p.validate().is_ok(), "{:?}", p.validate());
}

#[test]
fn validate_empty_manufacturer_name() {
    let mut p = make_passport();
    p.manufacturer.name = "   ".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("manufacturer.name"), "got: {err}");
}

#[test]
fn validate_empty_manufacturer_address() {
    let mut p = make_passport();
    p.manufacturer.address = "".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("manufacturer.address"), "got: {err}");
}

#[test]
fn validate_invalid_semver() {
    let mut p = make_passport();
    p.schema_version = "v1".to_owned();
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("schema_version"), "got: {err}");
}

#[test]
fn validate_rejects_vacuous_semver() {
    // ".5.0" (empty major) and "1.0.abc" (non-numeric patch) previously slipped
    // past the hand-rolled digit check and then silently skipped schema
    // validation downstream.
    for bad in [".5.0", "1.0.abc", "1.0", "1"] {
        let mut p = make_passport();
        p.schema_version = bad.to_owned();
        let err = p.validate().unwrap_err().to_string();
        assert!(
            err.contains("schema_version"),
            "schema_version '{bad}' should be rejected, got: {err}"
        );
    }

    // Pre-release / build metadata are still valid semver and must be accepted.
    let mut p = make_passport();
    p.schema_version = "1.0.0-alpha".to_owned();
    assert!(
        p.validate().is_ok(),
        "pre-release semver should be accepted"
    );
}

#[test]
fn validate_negative_co2e() {
    let mut p = make_passport();
    p.co2e_per_unit = Some(CarbonFootprint::from_kg(-1.0));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("co2e_per_unit"), "got: {err}");
}

#[test]
fn validate_repairability_out_of_range() {
    let mut p = make_passport();
    p.repairability_score = Some(RepairabilityScore::from_scalar(11.0));
    let err = p.validate().unwrap_err().to_string();
    assert!(err.contains("repairability_score"), "got: {err}");
}

#[test]
fn validate_multiple_errors_joined() {
    let mut p = make_passport();
    p.product_name = "".to_owned();
    p.manufacturer.name = "".to_owned();
    p.co2e_per_unit = Some(CarbonFootprint::from_kg(-5.0));
    let err = p.validate().unwrap_err().to_string();
    // All three issues should appear, separated by semicolons
    assert!(err.contains("product_name"), "got: {err}");
    assert!(err.contains("manufacturer.name"), "got: {err}");
    assert!(err.contains("co2e_per_unit"), "got: {err}");
}

#[test]
fn validate_none_optionals_ok() {
    let mut p = make_passport();
    p.co2e_per_unit = None;
    p.repairability_score = None;
    assert!(p.validate().is_ok());
}

#[test]
fn v02_fields_round_trip() {
    let mut p = make_passport();
    let predecessor_id = PassportId(Uuid::now_v7());
    p.version = 2;
    p.supersedes_id = Some(predecessor_id);
    p.retention_until = Some(Utc::now() + chrono::Duration::days(3650));
    p.product_id = Some(Uuid::now_v7());
    p.operator_identifier = Some("DE12345678".to_owned());
    p.facility = Some(crate::domain::passport::FacilitySnapshot {
        scheme: "national".to_owned(),
        value: "FAC-001".to_owned(),
        name: "Plant One".to_owned(),
        country: "DE".to_owned(),
        address: None,
    });

    let json = serde_json::to_string(&p).unwrap();
    let back: Passport = serde_json::from_str(&json).unwrap();
    assert_eq!(back.version, 2);
    assert_eq!(back.supersedes_id, Some(predecessor_id));
    assert!(back.retention_until.is_some());
    assert_eq!(back.operator_identifier.as_deref(), Some("DE12345678"));
    assert_eq!(
        back.facility.as_ref().map(|f| f.value.as_str()),
        Some("FAC-001")
    );
    assert_eq!(
        back.facility.as_ref().map(|f| f.name.as_str()),
        Some("Plant One")
    );
}

#[test]
fn published_to_superseded_is_valid_transition() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Superseded).unwrap();
    assert_eq!(p.status, PassportStatus::Superseded);
}

#[test]
fn superseded_is_terminal() {
    let mut p = make_passport();
    p.transition_to(PassportStatus::Published).unwrap();
    p.transition_to(PassportStatus::Superseded).unwrap();
    assert!(p.transition_to(PassportStatus::Published).is_err());
    assert!(p.transition_to(PassportStatus::Archived).is_err());
    assert!(p.transition_to(PassportStatus::Draft).is_err());
}

#[test]
fn default_version_is_one_and_skipped_when_none_optional_fields_absent() {
    let p = make_passport();
    let json = serde_json::to_value(&p).unwrap();
    assert_eq!(json["version"], 1);
    assert!(json.get("supersedes_id").is_none() || json["supersedes_id"].is_null());
    assert!(json.get("retentionUntil").is_none() || json["retentionUntil"].is_null());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_wires_product_group_data_validation() {
    use crate::domain::product_group::{FibreEntry, TextileData};
    let mut p = make_passport();
    p.product_group = ProductGroup::Textile;
    p.product_group_data = Some(ProductGroupData::Textile(Box::new(TextileData {
        // fibre sum = 50% — cross-field rule must catch this
        fibre_composition: vec![FibreEntry {
            fibre: "cotton".into(),
            pct: 50.0,
            country_of_origin: None,
        }],
        country_of_origin: "DE".into(),
        chemical_compliance_standard: "REACH".into(),
        ..crate::test_support::sample_textile_data()
    })));
    let err = p.validate().unwrap_err().to_string();
    assert!(
        err.contains("fibreComposition") || err.contains("fibre"),
        "expected fibre error from product_group_data validation, got: {err}"
    );
}

#[test]
fn product_group_data_preserved_round_trip() {
    let mut passport = make_passport();
    passport.product_group = ProductGroup::Battery; // keep product_group consistent with the data
    passport.product_group_data = Some(ProductGroupData::Battery(Box::new(BatteryData {
        state_of_health_pct: Some(95.3),
        rated_capacity_kwh: Some(32.0),
        ..crate::test_support::sample_battery_data()
    })));
    let json = serde_json::to_string(&passport).unwrap();
    let back: Passport = serde_json::from_str(&json).unwrap();
    if let Some(ProductGroupData::Battery(ref b)) = back.product_group_data {
        assert_eq!(b.battery_chemistry, BatteryChemistry::Lfp);
        assert_eq!(b.state_of_health_pct, Some(95.3));
    } else {
        panic!("expected Battery product_group data");
    }
}
