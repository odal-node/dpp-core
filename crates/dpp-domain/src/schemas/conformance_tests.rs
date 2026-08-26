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

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_unsold_goods_v2_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "2.0.0".parse().unwrap();
    let data = serde_json::json!({
        "entity": {
            "name": "Example Retail Group SA",
            "identifier": { "type": "euid", "value": "LUB123456789" },
            "scope": { "type": "standalone" }
        },
        "financialYear": { "start": "2027-01-01", "end": "2027-12-31" },
        "lines": [{
            "cnCategories": ["6203"],
            "description": "Men's suits and trousers",
            "unitsDiscarded": { "value": 1200 },
            "weightKg": { "value": 430, "estimated": true },
            "packagingIncluded": false,
            "reason": "damagedOrContaminated",
            "treatment": {
                "preparingForReusePct": 20, "recyclingPct": 50,
                "otherRecoveryPct": 20, "disposalPct": 5, "unknownPct": 5
            }
        }],
        "measuresTaken": "Pre-season demand forecasting.",
        "measuresPlanned": "Twelve-week donation window."
    });
    assert!(reg.validate("unsold-goods", &v, &data).is_ok());
}

/// The reason vocabulary is Del. Reg. (EU) 2026/296 Art. 2's derogation list.
/// `end_of_season` was ours and is not a derogation at all — a disclosure using
/// it claimed a lawful destruction the act does not permit.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_unsold_goods_v2_rejects_a_reason_outside_article_2() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "2.0.0".parse().unwrap();
    let mut data = serde_json::json!({
        "entity": {
            "name": "Example Retail Group SA",
            "identifier": { "type": "euid", "value": "LUB123456789" },
            "scope": { "type": "standalone" }
        },
        "financialYear": { "start": "2027-01-01", "end": "2027-12-31" },
        "lines": [{
            "cnCategories": ["6203"],
            "description": "Men's suits and trousers",
            "unitsDiscarded": { "value": 1200 },
            "weightKg": { "value": 430 },
            "packagingIncluded": false,
            "reason": "damagedOrContaminated",
            "treatment": {
                "preparingForReusePct": 20, "recyclingPct": 50,
                "otherRecoveryPct": 20, "disposalPct": 5, "unknownPct": 5
            }
        }],
        "measuresTaken": "Pre-season demand forecasting.",
        "measuresPlanned": "Twelve-week donation window."
    });
    data["lines"][0]["reason"] = serde_json::json!("end_of_season");
    assert!(reg.validate("unsold-goods", &v, &data).is_err());
}

/// Art. 3 delimits by CN chapter or heading; a product's own 6/8/10-digit code
/// is a different level of the nomenclature and files a whole chapter's goods
/// under one article.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_unsold_goods_v2_rejects_a_full_commodity_code_as_a_category() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "2.0.0".parse().unwrap();
    let mut data = serde_json::json!({
        "entity": {
            "name": "Example Retail Group SA",
            "identifier": { "type": "euid", "value": "LUB123456789" },
            "scope": { "type": "standalone" }
        },
        "financialYear": { "start": "2027-01-01", "end": "2027-12-31" },
        "lines": [{
            "cnCategories": ["6203"],
            "description": "Men's suits and trousers",
            "unitsDiscarded": { "value": 1200 },
            "weightKg": { "value": 430 },
            "packagingIncluded": false,
            "reason": "damagedOrContaminated",
            "treatment": {
                "preparingForReusePct": 20, "recyclingPct": 50,
                "otherRecoveryPct": 20, "disposalPct": 5, "unknownPct": 5
            }
        }],
        "measuresTaken": "Pre-season demand forecasting.",
        "measuresPlanned": "Twelve-week donation window."
    });
    data["lines"][0]["cnCategories"] = serde_json::json!(["62034231"]);
    assert!(reg.validate("unsold-goods", &v, &data).is_err());
}
