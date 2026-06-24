use super::*;

// ── Embedded schema tests ─────────────────────────────────────────────

#[test]
fn registry_loads_all_embedded_schemas() {
    let reg = VersionedSchemaRegistry::new();
    // battery 1.0 + 2.0, textile 1.0 + 1.1, textile-unsold 1.0, steel 1.0,
    // electronics 1.0, construction 1.0, tyre 1.0, toy 1.0, aluminium 1.0,
    // furniture 1.0, detergent 1.0
    assert_eq!(reg.len(), 13);
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
            "textile-unsold",
            "toy",
            "tyre",
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
    assert_eq!(reg.len(), 14);

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
    assert_eq!(reg.len(), 14);
}

#[test]
fn register_or_replace_existing_returns_true() {
    let mut reg = VersionedSchemaRegistry::new();
    let new_schema = r#"{"type": "object", "title": "updated"}"#;
    let replaced = reg
        .register_or_replace("battery", "1.0.0", new_schema.to_owned())
        .unwrap();
    assert!(replaced);
    assert_eq!(reg.len(), 13); // count unchanged
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
    assert_eq!(reg.len(), 14);

    let removed = reg.unregister("plastics", &"1.0.0".parse().unwrap());
    assert!(removed);
    assert_eq!(reg.len(), 13);
    assert!(reg.get("plastics", &"1.0.0".parse().unwrap()).is_none());
}

#[test]
fn unregister_embedded_schema_does_nothing() {
    let mut reg = VersionedSchemaRegistry::new();
    let removed = reg.unregister("battery", &"1.0.0".parse().unwrap());
    assert!(!removed);
    assert_eq!(reg.len(), 13); // still there
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
