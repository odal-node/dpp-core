//! Conformance for the product groups whose invalid fixture tests a rule its own
//! act imposes — a numeric sign, a country pattern, an enumerated reason list, a
//! CN category depth. One valid fixture and one targeted invalid fixture each.

use super::*;
use semver::Version;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_battery_v2_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "2.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 3.2,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4
    });
    assert!(reg.validate("battery", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_battery_v2_invalid_negative_co2e() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "2.0.0".parse().unwrap();
    // co2ePerUnitKg has minimum: 0 — negative value must be rejected.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "batteryChemistry": "NMC",
        "nominalVoltageV": 3.6,
        "nominalCapacityAh": 50.0,
        "expectedLifetimeCycles": 1000,
        "co2ePerUnitKg": -1.0
    });
    assert!(reg.validate("battery", &v, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_textile_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "fibreComposition": [{"fibre": "cotton", "pct": 100.0}],
        "countryOfManufacturing": "MK",
        "careInstructions": "Machine wash 30°C",
        "chemicalComplianceStandard": "OEKO-TEX 100"
    });
    assert!(reg.validate("textile", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_textile_v1_invalid_country_pattern() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // countryOfManufacturing must match ^[A-Z]{2}$ — lowercase fails.
    let data = serde_json::json!({
        "fibreComposition": [{"fibre": "cotton", "pct": 100.0}],
        "countryOfManufacturing": "macedonian",
        "careInstructions": "Hand wash",
        "chemicalComplianceStandard": "REACH"
    });
    assert!(reg.validate("textile", &v, &data).is_err());
}
