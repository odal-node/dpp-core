//! Schema + cross-field validation tests, including `SectorValidatorRegistry`
//! extensibility and batch validation.

use super::*;
use crate::domain::field_error::FieldError;
use crate::domain::sector::{BatteryData, FibreEntry, SectorData, TextileData};
use crate::schemas::VersionedSchemaRegistry;
use semver::Version;

fn valid_battery() -> SectorData {
    SectorData::Battery(BatteryData {
        nominal_voltage_v: 48.0,
        ..crate::test_support::sample_battery_data()
    })
}

fn valid_textile() -> SectorData {
    SectorData::Textile(TextileData {
        fibre_composition: vec![
            FibreEntry {
                fibre: "cotton".into(),
                pct: 60.0,
                country_of_origin: None,
            },
            FibreEntry {
                fibre: "polyester".into(),
                pct: 40.0,
                country_of_origin: None,
            },
        ],
        country_of_origin: "BD".into(),
        care_instructions: "30°C machine wash".into(),
        chemical_compliance_standard: "OEKO-TEX 100".into(),
        ..crate::test_support::sample_textile_data()
    })
}

#[test]
fn valid_battery_passes() {
    // Routed through the registry at the catalog's current battery version (v2.0.0).
    assert!(validate_sector_data(&valid_battery()).is_ok());
}

fn battery_inner() -> BatteryData {
    match valid_battery() {
        SectorData::Battery(b) => b,
        _ => unreachable!("valid_battery is Battery"),
    }
}

#[test]
fn battery_positive_cobalt_on_lfp_fails_cross_field() {
    let mut b = battery_inner(); // chemistry = LFP (no cobalt)
    b.recycled_content_cobalt_pct = Some(5.0);
    let err = validate_sector_data(&SectorData::Battery(b)).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| e.field == "/recycledContentCobaltPct"),
        "expected cobalt-on-LFP conflict, got: {err:?}"
    );
}

#[test]
fn battery_zero_cobalt_on_lfp_passes() {
    let mut b = battery_inner();
    b.recycled_content_cobalt_pct = Some(0.0); // "no recycled cobalt" — not a conflict
    b.recycled_content_lithium_pct = Some(12.5);
    assert!(validate_sector_data(&SectorData::Battery(b)).is_ok());
}

#[test]
fn battery_inverted_operating_temp_fails_cross_field() {
    let mut b = battery_inner();
    b.operating_temp_min_c = Some(60.0);
    b.operating_temp_max_c = Some(-20.0);
    let err = validate_sector_data(&SectorData::Battery(b)).unwrap_err();
    assert!(
        err.errors.iter().any(|e| e.field == "/operatingTempMinC"),
        "expected operating-temp conflict, got: {err:?}"
    );
}

#[test]
fn valid_textile_passes() {
    assert!(validate_sector_data(&valid_textile()).is_ok());
}

// The following exercise the schema layer directly through the registry,
// crafting structurally invalid instances the type system would otherwise
// prevent.

#[test]
fn battery_missing_required_field_fails() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let instance = serde_json::json!({
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4
        // "gtin" intentionally missing
    });
    let err = reg.validate("battery", &v, &instance).unwrap_err();
    assert!(
        err.errors.iter().any(|e| e.message.contains("gtin")),
        "expected gtin error, got: {err:?}"
    );
}

#[test]
fn battery_invalid_gtin_pattern_fails() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let instance = serde_json::json!({
        "gtin": "123", // too short
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4
    });
    assert!(reg.validate("battery", &v, &instance).is_err());
}

#[test]
fn textile_missing_care_instructions_fails() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.1.0".parse().unwrap();
    let instance = serde_json::json!({
        "fibreComposition": [{"fibre": "cotton", "pct": 100}],
        "countryOfManufacturing": "BD",
        // "careInstructions" intentionally missing
        "chemicalComplianceStandard": "REACH"
    });
    let err = reg.validate("textile", &v, &instance).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| e.message.contains("careInstructions")),
        "expected careInstructions error, got: {err:?}"
    );
}

#[test]
fn textile_empty_fibre_composition_fails() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.1.0".parse().unwrap();
    let instance = serde_json::json!({
        "fibreComposition": [], // minItems: 1
        "countryOfManufacturing": "DE",
        "careInstructions": "dry clean only",
        "chemicalComplianceStandard": "GOTS"
    });
    assert!(reg.validate("textile", &v, &instance).is_err());
}

#[test]
fn textile_fibre_sum_not_100_fails() {
    // Schema passes (pct 0–100 individually); the cross-field rule fails.
    let data = SectorData::Textile(TextileData {
        fibre_composition: vec![
            FibreEntry {
                fibre: "cotton".into(),
                pct: 60.0,
                country_of_origin: None,
            },
            FibreEntry {
                fibre: "polyester".into(),
                pct: 30.0, // sums to 90
                country_of_origin: None,
            },
        ],
        care_instructions: "Hand wash only".into(),
        chemical_compliance_standard: "REACH".into(),
        ..crate::test_support::sample_textile_data()
    });
    let err = validate_sector_data(&data).unwrap_err();
    assert!(
        err.errors.iter().any(|e| e.field == "/fibreComposition"),
        "expected /fibreComposition error, got: {err:?}"
    );
}

// ── SectorValidatorRegistry / validate_raw_sector_data tests ─────────────

#[test]
fn other_sector_data_fails_without_registry() {
    let data = SectorData::Other(serde_json::json!({"field": "value"}));
    let err = validate_sector_data(&data).unwrap_err();
    assert!(
        err.errors.iter().any(|e| e.field == "/sector"),
        "expected /sector error for Other without registry"
    );
}

#[test]
fn other_sector_data_passes_with_registered_validator() {
    use std::sync::Arc;

    struct AlwaysOkValidator;
    impl SectorValidator for AlwaysOkValidator {
        fn validate(&self, _: &serde_json::Value) -> Result<(), Vec<FieldError>> {
            Ok(())
        }
    }

    let mut registry = SectorValidatorRegistry::new();
    registry.register("other", Arc::new(AlwaysOkValidator));

    let data = SectorData::Other(serde_json::json!({"field": "value"}));
    assert!(
        validate_sector_data_with_registry(&data, &registry).is_ok(),
        "registered AlwaysOkValidator must allow Other sector"
    );
}

#[test]
fn other_sector_data_validator_errors_propagate() {
    use std::sync::Arc;

    struct AlwaysFailValidator;
    impl SectorValidator for AlwaysFailValidator {
        fn validate(&self, _: &serde_json::Value) -> Result<(), Vec<FieldError>> {
            Err(vec![FieldError {
                field: "/field".to_owned(),
                message: "injected failure".to_owned(),
            }])
        }
    }

    let mut registry = SectorValidatorRegistry::new();
    registry.register("other", Arc::new(AlwaysFailValidator));

    let data = SectorData::Other(serde_json::json!({"field": "bad"}));
    let err = validate_sector_data_with_registry(&data, &registry).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| e.message.contains("injected failure")),
        "validator errors must propagate"
    );
}

#[test]
fn validate_raw_sector_data_known_sector_succeeds() {
    // "battery" has an embedded schema — validate known-good raw JSON.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4
    });
    let registry = SectorValidatorRegistry::default();
    assert!(validate_raw_sector_data("battery", &data, &registry).is_ok());
}

#[test]
fn validate_raw_sector_data_unknown_sector_fails() {
    let data = serde_json::json!({"field": "value"});
    let registry = SectorValidatorRegistry::default();
    let err = validate_raw_sector_data("nonexistent-sector", &data, &registry).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| e.message.contains("nonexistent-sector")),
        "expected error naming the unknown sector key"
    );
}

#[test]
fn batch_validation_mixed_results() {
    let items = vec![
        valid_battery(),
        valid_textile(),
        // Invalid: fibre sum != 100
        SectorData::Textile(TextileData {
            fibre_composition: vec![FibreEntry {
                fibre: "cotton".into(),
                pct: 50.0,
                country_of_origin: None,
            }],
            care_instructions: "Hand wash".into(),
            chemical_compliance_standard: "REACH".into(),
            ..crate::test_support::sample_textile_data()
        }),
    ];

    let results = validate_sector_data_batch(&items);
    assert_eq!(results.len(), 3);
    assert!(results[0].result.is_ok());
    assert!(results[1].result.is_ok());
    assert!(results[2].result.is_err());

    let errors = batch_errors(&results);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].index, 2);
}
