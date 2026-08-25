//! Schema + cross-field validation tests, including `ProductGroupValidatorRegistry`
//! extensibility and batch validation.

use super::*;
use crate::domain::product_group::{BatteryData, FibreEntry, ProductGroupData, TextileData};
use crate::error::field::FieldError;
use crate::schemas::VersionedSchemaRegistry;
use semver::Version;

fn valid_battery() -> ProductGroupData {
    ProductGroupData::Battery(Box::new(BatteryData {
        nominal_voltage_v: 48.0,
        ..crate::test_support::sample_battery_data()
    }))
}

fn valid_textile() -> ProductGroupData {
    ProductGroupData::Textile(Box::new(TextileData {
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
    }))
}

#[test]
fn valid_battery_passes() {
    // Routed through the registry at the catalog's current battery version (v2.0.0).
    assert!(validate_product_group_data(&valid_battery()).is_ok());
}

fn battery_inner() -> BatteryData {
    match valid_battery() {
        ProductGroupData::Battery(b) => *b,
        _ => unreachable!("valid_battery is Battery"),
    }
}

#[test]
fn battery_positive_cobalt_on_lfp_fails_cross_field() {
    let mut b = battery_inner(); // chemistry = LFP (no cobalt)
    b.recycled_content_cobalt_pct = Some(5.0);
    let err = validate_product_group_data(&ProductGroupData::Battery(Box::new(b))).unwrap_err();
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
    assert!(validate_product_group_data(&ProductGroupData::Battery(Box::new(b))).is_ok());
}

#[test]
fn battery_inverted_operating_temp_fails_cross_field() {
    let mut b = battery_inner();
    b.operating_temp_min_c = Some(60.0);
    b.operating_temp_max_c = Some(-20.0);
    let err = validate_product_group_data(&ProductGroupData::Battery(Box::new(b))).unwrap_err();
    assert!(
        err.errors.iter().any(|e| e.field == "/operatingTempMinC"),
        "expected operating-temp conflict, got: {err:?}"
    );
}

#[test]
fn valid_textile_passes() {
    assert!(validate_product_group_data(&valid_textile()).is_ok());
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
    let data = ProductGroupData::Textile(Box::new(TextileData {
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
    }));
    let err = validate_product_group_data(&data).unwrap_err();
    assert!(
        err.errors.iter().any(|e| e.field == "/fibreComposition"),
        "expected /fibreComposition error, got: {err:?}"
    );
}

// ── ProductGroupValidatorRegistry / validate_raw_product_group_data tests ─────────────

#[test]
fn other_product_group_data_fails_without_registry() {
    let data = ProductGroupData::other(serde_json::json!({"field": "value"})).expect("untyped");
    let err = validate_product_group_data(&data).unwrap_err();
    assert!(
        err.errors.iter().any(|e| e.field == "/product_group"),
        "expected /product_group error for Other without registry"
    );
}

#[test]
fn other_product_group_data_passes_with_registered_validator() {
    use std::sync::Arc;

    struct AlwaysOkValidator;
    impl ProductGroupValidator for AlwaysOkValidator {
        fn validate(&self, _: &serde_json::Value) -> Result<(), Vec<FieldError>> {
            Ok(())
        }
    }

    let mut registry = ProductGroupValidatorRegistry::new();
    registry.register("other", Arc::new(AlwaysOkValidator));

    let data = ProductGroupData::other(serde_json::json!({"field": "value"})).expect("untyped");
    assert!(
        validate_product_group_data_with_registry(&data, &registry).is_ok(),
        "registered AlwaysOkValidator must allow Other product_group"
    );
}

#[test]
fn other_product_group_data_validator_errors_propagate() {
    use std::sync::Arc;

    struct AlwaysFailValidator;
    impl ProductGroupValidator for AlwaysFailValidator {
        fn validate(&self, _: &serde_json::Value) -> Result<(), Vec<FieldError>> {
            Err(vec![FieldError {
                field: "/field".to_owned(),
                message: "injected failure".to_owned(),
            }])
        }
    }

    let mut registry = ProductGroupValidatorRegistry::new();
    registry.register("other", Arc::new(AlwaysFailValidator));

    let data = ProductGroupData::other(serde_json::json!({"field": "bad"})).expect("untyped");
    let err = validate_product_group_data_with_registry(&data, &registry).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| e.message.contains("injected failure")),
        "validator errors must propagate"
    );
}

#[test]
fn validate_raw_product_group_data_known_product_group_succeeds() {
    // "battery" has an embedded schema — validate known-good raw JSON.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "batteryType": "portable"
    });
    let registry = ProductGroupValidatorRegistry::default();
    assert!(validate_raw_product_group_data("battery", &data, &registry).is_ok());
}

#[test]
fn validate_raw_product_group_data_unknown_product_group_fails() {
    let data = serde_json::json!({"field": "value"});
    let registry = ProductGroupValidatorRegistry::default();
    let err =
        validate_raw_product_group_data("nonexistent-product_group", &data, &registry).unwrap_err();
    assert!(
        err.errors
            .iter()
            .any(|e| e.message.contains("nonexistent-product_group")),
        "expected error naming the unknown product_group key"
    );
}

#[test]
fn batch_validation_mixed_results() {
    let items = vec![
        valid_battery(),
        valid_textile(),
        // Invalid: fibre sum != 100
        ProductGroupData::Textile(Box::new(TextileData {
            fibre_composition: vec![FibreEntry {
                fibre: "cotton".into(),
                pct: 50.0,
                country_of_origin: None,
            }],
            care_instructions: "Hand wash".into(),
            chemical_compliance_standard: "REACH".into(),
            ..crate::test_support::sample_textile_data()
        })),
    ];

    let results = validate_product_group_data_batch(&items);
    assert_eq!(results.len(), 3);
    assert!(results[0].result.is_ok());
    assert!(results[1].result.is_ok());
    assert!(results[2].result.is_err());

    let errors = batch_errors(&results);
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].index, 2);
}

// ── untyped product groups keep their embedded schema ────────────────────────────────

/// A valid aluminium payload, as it would arrive from a build with no
/// `ProductGroupData::Aluminium` variant.
fn untyped_aluminium(recycled: serde_json::Value) -> ProductGroupData {
    ProductGroupData::Other {
        product_group: "aluminium".to_owned(),
        data: serde_json::json!({
            "gtin": "09506000134352",
            "alloyGrade": "6061",
            "productionRoute": "secondary-recycled",
            "co2ePerTonneKg": 4200.0,
            "recycledContentPct": recycled,
            "countryOfOrigin": "DE",
        }),
    }
}

/// A product group carried as `Other` is validated against the schema this crate
/// embeds for it, with an empty registry.
///
/// This is what makes removing a typed lane safe: the schema ships as a data
/// file and the typed variant is Rust code, so losing the second must not lose
/// the first. Before this, an untyped product group was rejected outright however good
/// its data was.
///
/// Constructed directly rather than deserialised, because every catalog product group
/// still has a typed variant today — which is exactly why the gap was latent.
#[test]
fn untyped_product_group_validates_against_its_embedded_schema() {
    let result = validate_product_group_data_with_registry(
        &untyped_aluminium(serde_json::json!(42.0)),
        &ProductGroupValidatorRegistry::default(),
    );
    assert!(
        result.is_ok(),
        "an untyped product_group with an embedded schema must validate, got {:?}",
        result.err()
    );
}

/// The fallback is not a pass-through: the embedded schema still rejects data
/// that violates it.
#[test]
fn untyped_product_group_schema_still_rejects_bad_data() {
    assert!(
        validate_product_group_data_with_registry(
            &untyped_aluminium(serde_json::json!("not a number")),
            &ProductGroupValidatorRegistry::default(),
        )
        .is_err(),
        "the embedded schema must still reject a type violation"
    );
}

/// A product group in neither the catalog nor the registry remains a hard error —
/// silent pass-through is still not safe.
#[test]
fn product_group_unknown_to_catalog_and_registry_is_still_refused() {
    let data = ProductGroupData::Other {
        product_group: "spacecraft".to_owned(),
        data: serde_json::json!({ "thrustKn": 12.0 }),
    };
    assert!(
        validate_product_group_data_with_registry(&data, &ProductGroupValidatorRegistry::default())
            .is_err(),
        "a product_group with neither a schema nor a validator must fail closed"
    );
}
