//! Conformance for the product groups whose invalid fixture tests plain shape —
//! enum membership, or a required field left out. One valid fixture and one
//! targeted invalid fixture each.

use super::*;
use semver::Version;

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_steel_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "co2ePerTonneSteel": 1.8,
        "recycledScrapContentPct": 35.0,
        "productCategory": "flat",
        "countryOfProduction": "DE",
        "productionRoute": "electric-arc"
    });
    assert!(reg.validate("steel", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_steel_v1_invalid_production_route_enum() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // "open-hearth" is not a valid productionRoute value.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "co2ePerTonneSteel": 2.5,
        "recycledScrapContentPct": 10.0,
        "productCategory": "long",
        "countryOfProduction": "UA",
        "productionRoute": "open-hearth"
    });
    assert!(reg.validate("steel", &v, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_electronics_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "productCategory": "smartphone",
        "energyEfficiencyClass": "A",
        "co2ePerUnitKg": 65.0
    });
    assert!(reg.validate("electronics", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_electronics_v1_invalid_efficiency_class_enum() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // "H" is not in the A-G energy efficiency class enum.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "productCategory": "laptop",
        "energyEfficiencyClass": "H",
        "co2ePerUnitKg": 200.0
    });
    assert!(reg.validate("electronics", &v, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_construction_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "productFamily": "cement",
        "countryOfManufacture": "DE",
        "co2ePerFunctionalUnitKg": 780.0,
        "functionalUnit": "per tonne"
    });
    assert!(reg.validate("construction", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_construction_v1_invalid_missing_functional_unit() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // functionalUnit is required — omitting it must be rejected.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "productFamily": "glass",
        "countryOfManufacture": "PL",
        "co2ePerFunctionalUnitKg": 5.2
    });
    assert!(reg.validate("construction", &v, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_tyre_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "tyreClass": "C1",
        "fuelEfficiencyClass": "A",
        "wetGripClass": "B",
        "externalRollingNoiseDb": 68.0
    });
    assert!(reg.validate("tyre", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_tyre_v1_invalid_old_scale_class() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // "F" was valid on the old A-G scale but is NOT valid on the 2021 A-E scale.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "tyreClass": "C1",
        "fuelEfficiencyClass": "F",
        "wetGripClass": "A",
        "externalRollingNoiseDb": 71.0
    });
    assert!(reg.validate("tyre", &v, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_toy_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "ageGroup": "3-6",
        "primaryMaterial": "wood",
        "ceMarking": true,
        "countryOfManufacture": "DE"
    });
    assert!(reg.validate("toy", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_toy_v1_invalid_missing_age_group() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // ageGroup is required — omitting it must be rejected.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "primaryMaterial": "plastic",
        "ceMarking": true,
        "countryOfManufacture": "CN"
    });
    assert!(reg.validate("toy", &v, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_aluminium_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "alloyGrade": "6xxx",
        "productionRoute": "primary",
        "co2ePerTonneKg": 8500.0,
        "recycledContentPct": 0.0,
        "countryOfProduction": "NO"
    });
    assert!(reg.validate("aluminium", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_aluminium_v1_invalid_production_route_enum() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // "secondary" is not valid; must be "secondary-recycled".
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "alloyGrade": "3xxx",
        "productionRoute": "secondary",
        "co2ePerTonneKg": 600.0,
        "recycledContentPct": 95.0,
        "countryOfProduction": "DE"
    });
    assert!(reg.validate("aluminium", &v, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_furniture_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "productType": "chair",
        "primaryMaterial": "solid-wood",
        "countryOfManufacture": "MK"
    });
    assert!(reg.validate("furniture", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_furniture_v1_invalid_product_type_enum() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // "desk" is not in the product type enum.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "productType": "desk",
        "primaryMaterial": "metal",
        "countryOfManufacture": "PL"
    });
    assert!(reg.validate("furniture", &v, &data).is_err());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_detergent_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "productType": "laundry",
        "format": "liquid",
        "surfactants": [
            {"name": "SLES", "biodegradable": true, "concentrationBand": "5-15%"}
        ],
        "countryOfManufacture": "DE"
    });
    assert!(reg.validate("detergent", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_detergent_v1_invalid_empty_surfactants() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // surfactants has minItems: 1 — empty array must be rejected.
    let data = serde_json::json!({
        "gtin": "09506000134352",
        "productType": "dishwashing",
        "format": "tablet",
        "surfactants": [],
        "countryOfManufacture": "FR"
    });
    assert!(reg.validate("detergent", &v, &data).is_err());
}
