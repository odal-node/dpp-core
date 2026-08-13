use super::*;
use semver::Version;

// ── Embedded schema tests ─────────────────────────────────────────────

#[test]
fn registry_loads_all_embedded_schemas() {
    let reg = VersionedSchemaRegistry::new();
    // battery 1.0 + 2.0 + 2.1 + 2.2 + 2.3 + 2.4 + 2.5 + 2.6, textile 1.0 + 1.1 + 1.2,
    // unsold-goods 1.0,
    // steel 1.0 + 1.1, electronics 1.0 + 1.1, construction 1.0 + 1.1,
    // tyre 1.0, toy 1.0 + 1.1, aluminium 1.0 + 1.1, furniture 1.0 + 1.1,
    // detergent 1.0 + 1.1
    assert_eq!(reg.len(), 27);
}

#[test]
fn get_battery_v1() {
    let reg = VersionedSchemaRegistry::new();
    let v1: Version = "1.0.0".parse().unwrap();
    let json = reg.get("battery", &v1);
    assert!(json.is_some());
    let parsed: serde_json::Value = serde_json::from_str(json.unwrap()).unwrap();
    assert!(parsed.is_object());
}

#[test]
fn latest_battery_returns_v2_6() {
    let reg = VersionedSchemaRegistry::new();
    let (version, _json) = reg.latest("battery").expect("battery schema exists");
    assert_eq!(*version, "2.6.0".parse::<Version>().unwrap());
}

#[test]
fn latest_textile_returns_v1_2() {
    let reg = VersionedSchemaRegistry::new();
    let (version, _json) = reg.latest("textile").expect("textile schema exists");
    assert_eq!(*version, "1.2.0".parse::<Version>().unwrap());
}

#[test]
fn get_nonexistent_sector_returns_none() {
    let reg = VersionedSchemaRegistry::new();
    let v1: Version = "1.0.0".parse().unwrap();
    assert!(reg.get("plastics", &v1).is_none());
}

#[test]
fn get_nonexistent_version_returns_none() {
    let reg = VersionedSchemaRegistry::new();
    let v99: Version = "99.0.0".parse().unwrap();
    assert!(reg.get("battery", &v99).is_none());
}

#[test]
fn sectors_returns_unique_sorted_list() {
    let reg = VersionedSchemaRegistry::new();
    let sectors = reg.sectors();
    assert_eq!(
        sectors,
        vec![
            "aluminium",
            "battery",
            "construction",
            "detergent",
            "electronics",
            "furniture",
            "steel",
            "textile",
            "toy",
            "tyre",
            "unsold-goods",
        ]
    );
}

#[test]
fn versions_for_textile_returns_all_three() {
    let reg = VersionedSchemaRegistry::new();
    let versions = reg.versions_for("textile");
    assert_eq!(versions.len(), 3);
    assert_eq!(*versions[0], "1.0.0".parse::<Version>().unwrap());
    assert_eq!(*versions[1], "1.1.0".parse::<Version>().unwrap());
    assert_eq!(*versions[2], "1.2.0".parse::<Version>().unwrap());
}

// ── Hot-reload / runtime registration tests ───────────────────────────

#[test]
fn register_new_schema_succeeds() {
    let mut reg = VersionedSchemaRegistry::new();
    let schema = r#"{"type": "object", "properties": {"gtin": {"type": "string"}}}"#;
    assert!(reg.register("plastics", "1.0.0", schema.to_owned()).is_ok());
    assert_eq!(reg.len(), 28);

    let entry = reg
        .get_entry("plastics", &"1.0.0".parse().unwrap())
        .unwrap();
    assert_eq!(entry.origin, SchemaOrigin::Runtime);
}

#[test]
fn register_duplicate_fails() {
    let mut reg = VersionedSchemaRegistry::new();
    let schema = r#"{"type": "object"}"#;
    // battery v1.0.0 already exists (embedded)
    let err = reg
        .register("battery", "1.0.0", schema.to_owned())
        .unwrap_err();
    assert!(matches!(err, SchemaRegistrationError::AlreadyExists { .. }));
}

#[test]
fn register_invalid_json_fails() {
    let mut reg = VersionedSchemaRegistry::new();
    let err = reg
        .register("plastics", "1.0.0", "not json {{{".to_owned())
        .unwrap_err();
    assert!(matches!(err, SchemaRegistrationError::InvalidJson(_)));
}

#[test]
fn register_invalid_version_fails() {
    let mut reg = VersionedSchemaRegistry::new();
    let err = reg
        .register("plastics", "not-a-version", r#"{}"#.to_owned())
        .unwrap_err();
    assert!(matches!(err, SchemaRegistrationError::InvalidVersion(_)));
}

#[test]
fn schema_registration_error_display() {
    let invalid_json = SchemaRegistrationError::InvalidJson("trailing comma".into());
    assert_eq!(
        invalid_json.to_string(),
        "invalid JSON schema: trailing comma"
    );

    let exists = SchemaRegistrationError::AlreadyExists {
        sector: "battery".into(),
        version: "1.0.0".parse().unwrap(),
    };
    assert_eq!(
        exists.to_string(),
        "schema already exists for battery v1.0.0"
    );

    let invalid_version = SchemaRegistrationError::InvalidVersion("v-bad".into());
    assert_eq!(invalid_version.to_string(), "invalid semver version: v-bad");
}

#[test]
fn register_or_replace_new_returns_false() {
    let mut reg = VersionedSchemaRegistry::new();
    let schema = r#"{"type": "object"}"#;
    let replaced = reg
        .register_or_replace("plastics", "1.0.0", schema.to_owned())
        .unwrap();
    assert!(!replaced);
    assert_eq!(reg.len(), 28);
}

#[test]
fn register_or_replace_existing_returns_true() {
    let mut reg = VersionedSchemaRegistry::new();
    let new_schema = r#"{"type": "object", "title": "updated"}"#;
    let replaced = reg
        .register_or_replace("battery", "1.0.0", new_schema.to_owned())
        .unwrap();
    assert!(replaced);
    assert_eq!(reg.len(), 27); // count unchanged
    assert!(
        reg.get("battery", &"1.0.0".parse().unwrap())
            .unwrap()
            .contains("updated")
    );
}

#[test]
fn register_bumps_latest() {
    let mut reg = VersionedSchemaRegistry::new();
    let schema = r#"{"type": "object", "title": "battery v3"}"#;
    reg.register("battery", "3.0.0", schema.to_owned()).unwrap();

    let (ver, json) = reg.latest("battery").unwrap();
    assert_eq!(*ver, "3.0.0".parse::<Version>().unwrap());
    assert!(json.contains("battery v3"));
}

#[test]
fn unregister_runtime_schema_succeeds() {
    let mut reg = VersionedSchemaRegistry::new();
    let schema = r#"{"type": "object"}"#;
    reg.register("plastics", "1.0.0", schema.to_owned())
        .unwrap();
    assert_eq!(reg.len(), 28);

    let removed = reg.unregister("plastics", &"1.0.0".parse().unwrap());
    assert!(removed);
    assert_eq!(reg.len(), 27);
    assert!(reg.get("plastics", &"1.0.0".parse().unwrap()).is_none());
}

#[test]
fn unregister_embedded_schema_does_nothing() {
    let mut reg = VersionedSchemaRegistry::new();
    let removed = reg.unregister("battery", &"1.0.0".parse().unwrap());
    assert!(!removed);
    assert_eq!(reg.len(), 27); // still there
}

#[test]
fn unregister_nonexistent_returns_false() {
    let mut reg = VersionedSchemaRegistry::new();
    let removed = reg.unregister("plastics", &"1.0.0".parse().unwrap());
    assert!(!removed);
}

// ── Validation tests ──────────────────────────────────────────────────

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
    // Unknown sector or unregistered version → skipped (Ok), not an error.
    assert!(
        reg.validate_if_present("no-such-sector", "1.0.0", &invalid)
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
    // G-3 (release review Report 5): the publish path uses `validate_strict`,
    // not `validate_if_present`, precisely so an unresolved schema or version
    // is a hard error rather than a silent skip (Q-2). This pins that contract
    // directly at the registry, independent of any handler/service wiring.
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

    // Unknown sector → Err (validate_if_present would skip this as Ok).
    assert!(
        reg.validate_strict("no-such-sector", "1.0.0", &invalid)
            .is_err()
    );
    // Known sector, unregistered version → Err (validate_if_present skips).
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

// ── G-8: Per-sector conformance fixtures ──────────────────────────────────────
//
// Each embedded sector schema gets one valid fixture (all required fields) and
// one invalid fixture (a targeted schema constraint that the Rust types alone
// do not enforce). Battery v1 and textile v1.1 are already covered above; these
// tests cover all remaining sector/version pairs.

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
fn conformance_unsold_goods_v1_valid() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    let data = serde_json::json!({
        "reportingPeriod": "2026-Q2",
        "volumeKg": 120.5,
        "productCategory": "apparel",
        "reason": "end_of_season",
        "destination": "donation",
        "countryOfDisposal": "DE"
    });
    assert!(reg.validate("unsold-goods", &v, &data).is_ok());
}

#[cfg(not(target_arch = "wasm32"))]
#[test]
fn conformance_unsold_goods_v1_invalid_destination_enum() {
    let reg = VersionedSchemaRegistry::new();
    let v: Version = "1.0.0".parse().unwrap();
    // "incineration" is not a valid destination enum value.
    let data = serde_json::json!({
        "reportingPeriod": "2026-Q2",
        "volumeKg": 50.0,
        "productCategory": "apparel",
        "reason": "end_of_season",
        "destination": "incineration",
        "countryOfDisposal": "DE"
    });
    assert!(reg.validate("unsold-goods", &v, &data).is_err());
}

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
    use crate::domain::sector::CarbonFootprintClass;

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

/// Every field the Rust type can emit is a field the current schema admits.
///
/// The current battery schema sets `additionalProperties: false`, so a struct
/// field with no schema property fails validation the moment it is populated.
/// That drift is invisible to any fixture built with `..base`: serde skips
/// `None`, so an unpopulated field never reaches the schema. It went unseen
/// from v2.0.0 to v2.6.0 — `manufacturingDate`, `manufacturingPlace`,
/// `batteryModelId` and `batteryPassportNumber` were emittable the whole time
/// and no schema version declared any of them, so a battery passport recording
/// when or where it was made could not be validated at all.
///
/// **The literal below is deliberately exhaustive — do not add `..base` to
/// it.** That is the entire mechanism: Rust requires every field in a struct
/// literal without a base, so adding a field to `BatteryData` stops this file
/// compiling until someone populates it here, and the assertion then forces
/// the matching schema property to exist. A base expression silently restores
/// the hole. There is no reflection available to do this at runtime; the
/// compiler is the only thing that can enforce completeness.
///
/// Resolves the version from the catalog rather than naming one, so a schema
/// bump does not need this test edited — and cannot quietly leave it asserting
/// against a superseded version.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn a_fully_populated_battery_serialises_into_the_current_schema() {
    use crate::domain::gtin::Gtin;
    use crate::domain::sector::{
        BatteryChemistry, BatteryData, BatteryStatus, BatteryType, CarbonFootprintClass,
        CriticalRawMaterial, DynamicPerformance, EnvironmentalReading, ExpectedLifetime,
        HarmfulEvents, HazardSymbol, HazardousSubstance, MaterialComposition, StateOfChargeReading,
        StateOfHealth, TemperatureRange, UsageHistory,
    };
    use chrono::{NaiveDate, TimeZone as _, Utc};

    let range = TemperatureRange {
        min_c: -20.0,
        max_c: 60.0,
    };
    let moment = Utc.with_ymd_and_hms(2026, 3, 1, 12, 0, 0).unwrap();
    let day = NaiveDate::from_ymd_opt(2026, 3, 1).unwrap();

    let data = BatteryData {
        gtin: Gtin::parse("09506000134352").expect("valid GTIN literal"),
        battery_chemistry: BatteryChemistry::Lfp,
        nominal_voltage_v: 3.2,
        nominal_capacity_ah: 100.0,
        expected_lifetime_cycles: Some(3_000),
        co2e_per_unit_kg: 85.4,
        recycled_content_cobalt_pct: Some(16.0),
        recycled_content_lithium_pct: Some(6.0),
        recycled_content_nickel_pct: Some(6.0),
        state_of_health_pct: Some(97.0),
        rated_capacity_kwh: Some(64.0),
        carbon_footprint_class: Some(CarbonFootprintClass::new("B").expect("valid label")),
        carbon_footprint_class_ruleset_id: Some("eu-battery-cfb".into()),
        carbon_footprint_class_ruleset_version: Some("2026.1".into()),
        due_diligence_url: Some("https://example.invalid/due-diligence".into()),
        cathode_material: Some(vec![MaterialComposition {
            name: "Lithium iron phosphate".into(),
            weight_pct: 32.0,
            cas_number: Some("15365-14-7".into()),
        }]),
        anode_material: Some(vec![MaterialComposition {
            name: "Graphite".into(),
            weight_pct: 18.0,
            cas_number: Some("7782-42-5".into()),
        }]),
        electrolyte_material: Some(vec![MaterialComposition {
            name: "Lithium hexafluorophosphate".into(),
            weight_pct: 11.0,
            cas_number: Some("21324-40-3".into()),
        }]),
        critical_raw_materials: Some(vec![CriticalRawMaterial {
            name: "Natural graphite".into(),
            cas_number: Some("7782-42-5".into()),
            weight_grams: Some(8_500.0),
            country_of_origin: Some("MZ".into()),
        }]),
        disassembly_instructions_url: Some("https://example.invalid/disassembly".into()),
        soh_methodology: Some("IEC 62660-1:2018".into()),
        operating_temp_min_c: Some(-20.0),
        operating_temp_max_c: Some(60.0),
        rated_energy_wh: Some(64_000.0),
        recycled_content_lead_pct: Some(0.0),
        battery_weight_kg: Some(384.0),
        battery_type: BatteryType::Ev,
        initial_round_trip_efficiency_pct: Some(96.0),
        round_trip_efficiency_at_half_cycle_life_pct: Some(91.0),
        round_trip_efficiency_pct: Some(96.0),
        internal_resistance_mohm: Some(1.4),
        internal_cell_resistance_mohm: Some(1.4),
        internal_pack_resistance_mohm: Some(38.0),
        placed_on_market_date: Some(day),
        manufacturing_date: Some(moment),
        manufacturing_place: Some("PL:Wrocław".into()),
        battery_model_id: Some("LFP-64-A".into()),
        battery_passport_number: Some("URN:UUID:6F1C9D2E-0000-4000-8000-000000000000".into()),
        expected_lifetime: Some(Box::new(ExpectedLifetime {
            put_into_service_date: Some(day),
            energy_throughput_kwh: 128_000.0,
            capacity_throughput_ah: 400_000.0,
            harmful_events: HarmfulEvents {
                deep_discharge_events: Some(2),
                hours_in_extreme_temperature: Some(14.5),
                hours_charging_in_extreme_temperature: Some(1.5),
            },
            full_equivalent_cycles: 2_000.0,
        })),
        recycled_content_reporting_year: Some(2026),
        state_of_health: Some(Box::new(StateOfHealth::ElectricVehicle { soce_pct: 97.0 })),
        hazardous_substances: Some(vec![HazardousSubstance {
            name: "Nickel sulfate".into(),
            cas_number: Some("7786-81-4".into()),
            concentration_pct: Some(0.4),
        }]),
        usable_extinguishing_agent: Some("Class D dry powder".into()),
        renewable_content_pct: Some(12.5),
        minimal_voltage_v: Some(2.5),
        maximum_voltage_v: Some(4.2),
        voltage_temperature_range: Some(range),
        original_power_capability_w: Some(150_000.0),
        power_limit_min_w: Some(1_000.0),
        power_limit_max_w: Some(180_000.0),
        power_temperature_range: Some(range),
        expected_lifetime_reference_test: Some("IEC 62660-1:2018".into()),
        capacity_threshold_for_exhaustion_pct: Some(80.0),
        not_in_use_temperature_range: Some(range),
        not_in_use_temperature_reference_test: Some("IEC 62660-1:2018 clause 7".into()),
        commercial_warranty_period_months: Some(96),
        cycle_life_test_c_rate: Some(1.0),
        marking_information: Some("Separate collection symbol applied".into()),
        hazard_symbol: Some(HazardSymbol::Cadmium),
        eu_declaration_of_conformity: Some("DoC-2027-0001".into()),
        waste_battery_information: Some("https://example.invalid/waste".into()),
        component_part_numbers: Some(vec!["MOD-A1".into(), "BMS-C7".into()]),
        spare_parts_contacts: Some("spares@example.invalid".into()),
        safety_measures: Some("Isolate at the service disconnect before handling.".into()),
        test_report_results: Some("https://example.invalid/test-report".into()),
        dynamic_performance: Some(Box::new(DynamicPerformance {
            rated_capacity_ah: Some(98.0),
            capacity_fade_pct: Some(2.0),
            power_w: Some(148_000.0),
            power_fade_pct: Some(1.3),
            internal_resistance_mohm: Some(1.5),
            internal_resistance_increase_pct: Some(7.1),
            round_trip_efficiency_pct: Some(94.0),
            round_trip_efficiency_fade_pct: Some(2.1),
            expected_lifetime_cycles: Some(2_900),
            expected_lifetime_years: Some(11.5),
        })),
        battery_status: Some(BatteryStatus::Original),
        usage_history: Some(Box::new(UsageHistory {
            charge_discharge_cycles: Some(412),
            negative_events: Some(vec!["over-temperature 2026-02-11".into()]),
            operating_conditions: Some(vec![EnvironmentalReading {
                recorded_at: moment,
                temperature_c: Some(31.5),
                note: Some("fast charge".into()),
            }]),
            state_of_charge: Some(vec![StateOfChargeReading {
                recorded_at: moment,
                state_of_charge_pct: 78.0,
            }]),
        })),
    };

    let registry = VersionedSchemaRegistry::new();
    let (version, _) = registry.latest("battery").expect("battery schema exists");
    let json = serde_json::to_value(&data).expect("serialises");
    registry
        .validate("battery", version, &json)
        .expect("every emitted field must be admitted by the current schema");
}
