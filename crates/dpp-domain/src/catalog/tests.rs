//! `ProductGroupCatalog` load, gating, and runtime registration.

use super::*;

#[test]
fn loads_all_embedded_manifests() {
    let catalog = ProductGroupCatalog::new();
    assert_eq!(catalog.len(), 12);
}

/// Determinability is a property of an act reaching a product group, so it is
/// asked of the instrument catalog. The three answers are unchanged from when
/// this was a per-product group flag — what changed is that each now names the act it
/// comes from.
#[test]
fn exactly_three_product_groups_have_a_binding_act() {
    let instruments = InstrumentCatalog::new();
    let product_groups = ProductGroupCatalog::new();
    let mut in_force: Vec<&str> = product_groups
        .all()
        .iter()
        .map(|d| d.key.as_str())
        .filter(|key| !instruments.determinable_for(key).is_empty())
        .collect();
    in_force.sort_unstable();
    assert_eq!(in_force, vec!["battery", "electronics", "unsold-goods"]);
}

#[test]
fn product_groups_with_no_binding_act_are_flagged_not_dropped() {
    let catalog = ProductGroupCatalog::new();
    let instruments = InstrumentCatalog::new();
    // All eight not-yet-adopted product groups are still present, just flagged.
    assert_eq!(catalog.len(), 12);
    for key in ["textile", "steel"] {
        assert!(catalog.get(key).is_some(), "'{key}' must still be listed");
        assert!(
            instruments.determinable_for(key).is_empty(),
            "'{key}' has no act binding it yet"
        );
    }
}

#[test]
fn battery_descriptor_is_complete() {
    let catalog = ProductGroupCatalog::new();
    let battery = catalog.get("battery").expect("battery in catalog");
    assert!(battery.schema_versions.contains(&"2.0.0".to_string()));
    assert!(battery.schema_versions.contains(&"2.1.0".to_string()));
    assert!(battery.schema_versions.contains(&"2.2.0".to_string()));
    assert!(battery.schema_versions.contains(&"2.3.0".to_string()));
    assert!(battery.schema_versions.contains(&"2.4.0".to_string()));
    assert!(battery.schema_versions.contains(&"2.5.0".to_string()));
    assert!(battery.schema_versions.contains(&"2.6.0".to_string()));
    // Current version is v2.6.0, which adds the Annex XIII point 4
    // individual-battery tier and relaxes expectedLifetimeCycles out of
    // required. Older versions stay registered so passports already validated
    // against them remain verifiable.
    assert_eq!(battery.current_schema_version, "2.6.0");
    assert_eq!(battery.plugin.as_deref(), Some("product-group-battery"));
}

#[test]
fn resolve_schema_version_new_vs_existing() {
    let catalog = ProductGroupCatalog::new();
    // New passport (stored = None) → catalog current version.
    assert_eq!(
        catalog.resolve_schema_version("battery", None).as_deref(),
        Some("2.6.0")
    );
    // Existing passport → its stored version is authoritative, even if old.
    assert_eq!(
        catalog
            .resolve_schema_version("battery", Some("1.0.0"))
            .as_deref(),
        Some("1.0.0")
    );
    // Unknown product group, new passport → None.
    assert_eq!(catalog.resolve_schema_version("unknown", None), None);
}

#[test]
fn determination_gating_is_status_driven() {
    let catalog = InstrumentCatalog::new();
    for key in ["battery", "unsold-goods", "electronics"] {
        assert!(!catalog.determinable_for(key).is_empty(), "'{key}'");
    }
    // Adopted but not yet applicable → flagged, and unknown keys reach nothing.
    assert!(catalog.determinable_for("detergent").is_empty());
    assert!(catalog.determinable_for("nonexistent").is_empty());
}

#[test]
fn allows_determination_matches_status() {
    assert!(RegulatoryStatus::InForce.allows_determination());
    assert!(!RegulatoryStatus::Provisional.allows_determination());
}

#[test]
fn register_runtime_product_group() {
    let mut catalog = ProductGroupCatalog::new();
    let descriptor = ProductGroupDescriptor {
        key: "plastics".into(),
        title: "Plastics".into(),
        schema_versions: vec!["1.0.0".into()],
        current_schema_version: "1.0.0".into(),
        product_categories: vec![],
        disclosure: std::collections::HashMap::new(),
        plugin: None,
        notes: None,
    };
    assert!(catalog.register(descriptor.clone()).is_ok());
    assert_eq!(catalog.len(), 13);
    assert!(matches!(
        catalog.register(descriptor),
        Err(CatalogError::AlreadyExists(_))
    ));
}

fn provisional_descriptor(current: &str, versions: Vec<String>) -> ProductGroupDescriptor {
    ProductGroupDescriptor {
        key: "plastics".into(),
        title: "Plastics".into(),
        schema_versions: versions,
        current_schema_version: current.into(),
        product_categories: vec![],
        disclosure: std::collections::HashMap::new(),
        plugin: None,
        notes: None,
    }
}

#[test]
fn register_rejects_invalid_current_schema_version() {
    let mut catalog = ProductGroupCatalog::new();
    let descriptor = provisional_descriptor("not-semver", vec!["not-semver".into()]);
    assert!(matches!(
        catalog.register(descriptor),
        Err(CatalogError::InvalidSchemaVersion { .. })
    ));
    // A rejected descriptor must never reach the catalog — otherwise every
    // passport in that product group silently skips schema validation.
    assert_eq!(catalog.len(), 12);
}

#[test]
fn register_rejects_current_version_not_in_list() {
    let mut catalog = ProductGroupCatalog::new();
    // Valid semver, but not one of the declared schema_versions.
    let descriptor = provisional_descriptor("2.0.0", vec!["1.0.0".into()]);
    assert!(matches!(
        catalog.register(descriptor),
        Err(CatalogError::CurrentVersionNotListed { .. })
    ));
    assert_eq!(catalog.len(), 12);
}
