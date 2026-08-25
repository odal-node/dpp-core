//! The `validate*` surface: strict versus if-present, runtime-registered schemas,
//! and the bounds a schema places on a Rust type.

use super::*;
use semver::Version;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_valid_battery_data() {
    let reg = VersionedSchemaRegistry::new();
    let v1: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "12345678901234",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4
    });
    assert!(reg.validate("battery", &v1, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_invalid_battery_data() {
    let reg = VersionedSchemaRegistry::new();
    let v1: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "batteryChemistry": "LFP"
        // missing required fields
    });
    assert!(reg.validate("battery", &v1, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_if_present_enforces_existing_schema_and_skips_absent() {
    let reg = VersionedSchemaRegistry::new();
    let valid = serde_json::json!({
        "gtin": "12345678901234",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4
    });
    // Existing schema → enforced.
    assert!(reg.validate_if_present("battery", "1.0.0", &valid).is_ok());
    let invalid = serde_json::json!({ "batteryChemistry": "LFP" });
    assert!(
        reg.validate_if_present("battery", "1.0.0", &invalid)
            .is_err()
    );
    // Unknown product group or unregistered version → skipped (Ok), not an error.
    assert!(
        reg.validate_if_present("no-such-product_group", "1.0.0", &invalid)
            .is_ok()
    );
    assert!(
        reg.validate_if_present("battery", "9.9.9", &invalid)
            .is_ok()
    );
    // Unparseable version → skipped.
    assert!(
        reg.validate_if_present("battery", "not-a-version", &invalid)
            .is_ok()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_strict_is_fail_closed_unlike_validate_if_present() {
    // The publish path uses `validate_strict`, not `validate_if_present`,
    // precisely so an unresolved schema or version is a hard error rather than a
    // silent skip. This pins that contract directly at the registry, independent
    // of any handler/service wiring.
    let reg = VersionedSchemaRegistry::new();
    let valid = serde_json::json!({
        "gtin": "12345678901234",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4
    });
    let invalid = serde_json::json!({ "batteryChemistry": "LFP" });

    // Existing schema + valid data → Ok, same as validate_if_present.
    assert!(reg.validate_strict("battery", "1.0.0", &valid).is_ok());
    // Existing schema + invalid data → Err, same as validate_if_present.
    assert!(reg.validate_strict("battery", "1.0.0", &invalid).is_err());

    // Unknown product group → Err (validate_if_present would skip this as Ok).
    assert!(
        reg.validate_strict("no-such-product_group", "1.0.0", &invalid)
            .is_err()
    );
    // Known product group, unregistered version → Err (validate_if_present skips).
    assert!(reg.validate_strict("battery", "9.9.9", &invalid).is_err());
    // Unparseable version string → Err (validate_if_present skips).
    assert!(
        reg.validate_strict("battery", "not-a-version", &invalid)
            .is_err()
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_textile_v1_1_with_new_fields() {
    let reg = VersionedSchemaRegistry::new();
    let v11: Version = "1.1.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "fibreComposition": [
            { "fibre": "cotton", "pct": 70.0, "countryOfOrigin": "IN" },
            { "fibre": "polyester", "pct": 30.0, "countryOfOrigin": "CN" }
        ],
        "countryOfManufacturing": "BD",
        "careInstructions": "Machine wash 40°C",
        "chemicalComplianceStandard": "OEKO-TEX 100",
        "durabilityScore": 7.5,
        "microplasticSheddingMgPerWash": 12.3,
        "expectedWashCycles": 50,
        "svhcSubstances": [
            {
                "casNumber": "80-05-7",
                "substanceName": "Bisphenol A",
                "concentrationPct": 0.15,
                "locationInProduct": "coating"
            }
        ],
        "disassemblyInstructions": "Remove buttons, separate layers",
        "sparePartsAvailable": true,
        "productWeightGrams": 250.0
    });
    assert!(reg.validate("textile", &v11, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_textile_v1_1_rejects_invalid_fibre_country() {
    let reg = VersionedSchemaRegistry::new();
    let v11: Version = "1.1.0".parse().unwrap();
    let data = serde_json::json!({
        "fibreComposition": [
            { "fibre": "cotton", "pct": 100.0, "countryOfOrigin": "india" }
        ],
        "countryOfManufacturing": "BD",
        "careInstructions": "Hand wash",
        "chemicalComplianceStandard": "REACH"
    });
    assert!(reg.validate("textile", &v11, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn validate_runtime_registered_schema() {
    let mut reg = VersionedSchemaRegistry::new();
    let schema = r#"{
        "type": "object",
        "required": ["material"],
        "properties": {
            "material": { "type": "string", "minLength": 1 }
        },
        "additionalProperties": false
    }"#;
    reg.register("plastics", "1.0.0", schema.to_owned())
        .unwrap();

    let v1: Version = "1.0.0".parse().unwrap();

    // Valid data
    let valid = serde_json::json!({ "material": "PET" });
    assert!(reg.validate("plastics", &v1, &valid).is_ok());

    // Invalid: missing required field
    let invalid = serde_json::json!({});
    assert!(reg.validate("plastics", &v1, &invalid).is_err());
}

#[test]
fn carbon_footprint_class_bound_matches_the_schema() {
    // Drift guard: the newtype's length bound and the schema's `maxLength` are
    // two statements of one fact. If they disagree, a value can pass schema
    // validation and fail typed deserialization (or the reverse), and the
    // failure surfaces far from either declaration.
    use crate::domain::product_group::CarbonFootprintClass;

    let reg = VersionedSchemaRegistry::new();
    let json = reg
        .get("battery", &"2.2.0".parse::<Version>().unwrap())
        .expect("battery v2.2.0 is embedded");
    let schema: serde_json::Value = serde_json::from_str(json).unwrap();
    let max_length = schema["properties"]["carbonFootprintClass"]["maxLength"]
        .as_u64()
        .expect("carbonFootprintClass must declare maxLength");

    assert_eq!(max_length as usize, CarbonFootprintClass::MAX_LEN);

    // And the field must not have regained an enumeration: Art. 7(2) defines no
    // labels and requires the class count to be reviewed every three years.
    assert!(
        schema["properties"]["carbonFootprintClass"]
            .get("enum")
            .is_none(),
        "carbonFootprintClass must not enumerate labels"
    );
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn schema_rejects_a_state_of_health_mixing_both_annex_vii_lists() {
    // Annex VII Part A gives EV batteries exactly one parameter. serde ignores
    // unknown fields, so the guarantee that an EV payload cannot smuggle in a
    // stationary parameter lives in the schema's `additionalProperties: false`
    // — this asserts it at the layer that actually enforces it.
    let reg = VersionedSchemaRegistry::new();
    let v = "2.2.0".parse::<Version>().unwrap();

    let base = serde_json::json!({
        "gtin": "12345678901231",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
    });

    let mut mixed = base.clone();
    mixed["stateOfHealth"] = serde_json::json!({
        "parameterSet": "electricVehicle",
        "socePct": 90.0,
        "ohmicResistanceMohm": 3.2,
    });
    assert!(
        reg.validate("battery", &v, &mixed).is_err(),
        "an EV parameter set must not carry stationary parameters"
    );

    let mut valid = base;
    valid["stateOfHealth"] = serde_json::json!({
        "parameterSet": "electricVehicle",
        "socePct": 90.0,
    });
    assert!(reg.validate("battery", &v, &valid).is_ok());
}
