//! Registry mechanics: what is embedded, what `get`/`latest`/`versions_for` answer,
//! and what `register`/`unregister` accept or refuse.

use super::*;
use semver::Version;

#[test]
fn registry_loads_all_embedded_schemas() {
    let reg = VersionedSchemaRegistry::new();
    // Derived from the embedded table rather than written out. The count and
    // the per-product-group list that used to sit here went stale the first
    // time a schema version landed, which is the whole failure this asserts
    // against: `new()` must load every embedded schema, whatever there are.
    assert_eq!(reg.len(), super::embedded::EMBEDDED.len());
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
fn get_nonexistent_product_group_returns_none() {
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
fn product_groups_returns_unique_sorted_list() {
    let reg = VersionedSchemaRegistry::new();
    let product_groups = reg.product_groups();
    assert_eq!(
        product_groups,
        vec![
            "aluminium",
            "battery",
            "construction",
            "detergent",
            "electronics",
            "furniture",
            "mattress",
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
    assert_eq!(reg.len(), super::embedded::EMBEDDED.len() + 1);

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
        product_group: "battery".into(),
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
    assert_eq!(reg.len(), super::embedded::EMBEDDED.len() + 1);
}

#[test]
fn register_or_replace_existing_returns_true() {
    let mut reg = VersionedSchemaRegistry::new();
    let new_schema = r#"{"type": "object", "title": "updated"}"#;
    let replaced = reg
        .register_or_replace("battery", "1.0.0", new_schema.to_owned())
        .unwrap();
    assert!(replaced);
    assert_eq!(reg.len(), super::embedded::EMBEDDED.len()); // count unchanged
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
    assert_eq!(reg.len(), super::embedded::EMBEDDED.len() + 1);

    let removed = reg.unregister("plastics", &"1.0.0".parse().unwrap());
    assert!(removed);
    assert_eq!(reg.len(), super::embedded::EMBEDDED.len());
    assert!(reg.get("plastics", &"1.0.0".parse().unwrap()).is_none());
}

#[test]
fn unregister_embedded_schema_does_nothing() {
    let mut reg = VersionedSchemaRegistry::new();
    let removed = reg.unregister("battery", &"1.0.0".parse().unwrap());
    assert!(!removed);
    assert_eq!(reg.len(), super::embedded::EMBEDDED.len()); // still there
}

#[test]
fn unregister_nonexistent_returns_false() {
    let mut reg = VersionedSchemaRegistry::new();
    let removed = reg.unregister("plastics", &"1.0.0".parse().unwrap());
    assert!(!removed);
}

// ── Validation tests ──────────────────────────────────────────────────
