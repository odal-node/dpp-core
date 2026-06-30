use super::*;

// ── Embedded schema tests ─────────────────────────────────────────────

#[test]
fn registry_loads_all_embedded_schemas() {
    let reg = VersionedSchemaRegistry::new();
    // battery 1.0 + 2.0, textile 1.0 + 1.1, unsold-goods 1.0, steel 1.0,
    // electronics 1.0 + 1.1, construction 1.0, tyre 1.0, toy 1.0,
    // aluminium 1.0, furniture 1.0, detergent 1.0
    assert_eq!(reg.len(), 14);
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
fn latest_battery_returns_v2() {
    let reg = VersionedSchemaRegistry::new();
    let (version, _json) = reg.latest("battery").expect("battery schema exists");
    assert_eq!(*version, "2.0.0".parse::<Version>().unwrap());
}

#[test]
fn latest_textile_returns_v1_1() {
    let reg = VersionedSchemaRegistry::new();
    let (version, _json) = reg.latest("textile").expect("textile schema exists");
    assert_eq!(*version, "1.1.0".parse::<Version>().unwrap());
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
fn versions_for_textile_returns_both() {
    let reg = VersionedSchemaRegistry::new();
    let versions = reg.versions_for("textile");
    assert_eq!(versions.len(), 2);
    assert_eq!(*versions[0], "1.0.0".parse::<Version>().unwrap());
    assert_eq!(*versions[1], "1.1.0".parse::<Version>().unwrap());
}

// ── Hot-reload / runtime registration tests ───────────────────────────

#[test]
fn register_new_schema_succeeds() {
    let mut reg = VersionedSchemaRegistry::new();
    let schema = r#"{"type": "object", "properties": {"gtin": {"type": "string"}}}"#;
    assert!(reg.register("plastics", "1.0.0", schema.to_owned()).is_ok());
    assert_eq!(reg.len(), 15);

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
    assert_eq!(reg.len(), 15);
}

#[test]
fn register_or_replace_existing_returns_true() {
    let mut reg = VersionedSchemaRegistry::new();
    let new_schema = r#"{"type": "object", "title": "updated"}"#;
    let replaced = reg
        .register_or_replace("battery", "1.0.0", new_schema.to_owned())
        .unwrap();
    assert!(replaced);
    assert_eq!(reg.len(), 14); // count unchanged
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
    assert_eq!(reg.len(), 15);

    let removed = reg.unregister("plastics", &"1.0.0".parse().unwrap());
    assert!(removed);
    assert_eq!(reg.len(), 14);
    assert!(reg.get("plastics", &"1.0.0".parse().unwrap()).is_none());
}

#[test]
fn unregister_embedded_schema_does_nothing() {
    let mut reg = VersionedSchemaRegistry::new();
    let removed = reg.unregister("battery", &"1.0.0".parse().unwrap());
    assert!(!removed);
    assert_eq!(reg.len(), 14); // still there
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
