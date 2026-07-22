//! `SectorCatalog` load, gating, registration, and cross-artifact parity tests.

use super::*;

#[test]
fn loads_all_embedded_manifests() {
    let catalog = SectorCatalog::new();
    assert_eq!(catalog.len(), 11);
}

#[test]
fn exactly_three_sectors_are_in_force() {
    let catalog = SectorCatalog::new();
    let mut in_force: Vec<&str> = catalog.in_force().iter().map(|d| d.key.as_str()).collect();
    in_force.sort_unstable();
    assert_eq!(in_force, vec!["battery", "electronics", "unsold-goods"]);
}

#[test]
fn provisional_sectors_are_flagged_not_dropped() {
    let catalog = SectorCatalog::new();
    // All eight not-yet-adopted sectors are still present, just flagged.
    assert_eq!(catalog.provisional().len(), 8);
    assert!(!catalog.is_in_force("textile"));
    assert!(!catalog.is_in_force("steel"));
}

#[test]
fn battery_descriptor_is_complete() {
    let catalog = SectorCatalog::new();
    let battery = catalog.get("battery").expect("battery in catalog");
    assert_eq!(battery.status, RegulatoryStatus::InForce);
    assert_eq!(battery.dpp_applies_from.as_deref(), Some("2027-02-18"));
    assert_eq!(battery.retention_years, 10);
    assert!(battery.schema_versions.contains(&"2.0.0".to_string()));
    // Current version is v2.0.0 (Annex XIII), not the older v1.0.0.
    assert_eq!(battery.current_schema_version, "2.0.0");
    assert_eq!(battery.plugin.as_deref(), Some("sector-battery"));
}

#[test]
fn resolve_schema_version_new_vs_existing() {
    let catalog = SectorCatalog::new();
    // New passport (stored = None) → catalog current version.
    assert_eq!(
        catalog.resolve_schema_version("battery", None).as_deref(),
        Some("2.0.0")
    );
    // Existing passport → its stored version is authoritative, even if old.
    assert_eq!(
        catalog
            .resolve_schema_version("battery", Some("1.0.0"))
            .as_deref(),
        Some("1.0.0")
    );
    // Unknown sector, new passport → None.
    assert_eq!(catalog.resolve_schema_version("unknown", None), None);
}

#[test]
fn in_force_gating_is_status_driven() {
    let catalog = SectorCatalog::new();
    assert!(catalog.is_in_force("battery"));
    assert!(catalog.is_in_force("unsold-goods"));
    assert!(catalog.is_in_force("electronics"));
    assert!(!catalog.is_in_force("detergent")); // partial → flagged
    assert!(!catalog.is_in_force("nonexistent"));
}

#[test]
fn allows_determination_matches_status() {
    assert!(RegulatoryStatus::InForce.allows_determination());
    assert!(!RegulatoryStatus::Provisional.allows_determination());
}

#[test]
fn register_runtime_sector() {
    let mut catalog = SectorCatalog::new();
    let descriptor = SectorDescriptor {
        key: "plastics".into(),
        title: "Plastics".into(),
        status: RegulatoryStatus::Provisional,
        legal_basis: vec!["ESPR Working Plan".into()],
        dpp_applies_from: None,
        retention_years: 10,
        schema_versions: vec!["1.0.0".into()],
        current_schema_version: "1.0.0".into(),
        product_categories: vec![],
        access_tiers: std::collections::HashMap::new(),
        plugin: None,
        notes: None,
    };
    assert!(catalog.register(descriptor.clone()).is_ok());
    assert_eq!(catalog.len(), 12);
    assert!(matches!(
        catalog.register(descriptor),
        Err(CatalogError::AlreadyExists(_))
    ));
}

fn provisional_descriptor(current: &str, versions: Vec<String>) -> SectorDescriptor {
    SectorDescriptor {
        key: "plastics".into(),
        title: "Plastics".into(),
        status: RegulatoryStatus::Provisional,
        legal_basis: vec!["ESPR Working Plan".into()],
        dpp_applies_from: None,
        retention_years: 10,
        schema_versions: versions,
        current_schema_version: current.into(),
        product_categories: vec![],
        access_tiers: std::collections::HashMap::new(),
        plugin: None,
        notes: None,
    }
}

#[test]
fn register_rejects_invalid_current_schema_version() {
    let mut catalog = SectorCatalog::new();
    let descriptor = provisional_descriptor("not-semver", vec!["not-semver".into()]);
    assert!(matches!(
        catalog.register(descriptor),
        Err(CatalogError::InvalidSchemaVersion { .. })
    ));
    // A rejected descriptor must never reach the catalog — otherwise every
    // passport in that sector silently skips schema validation.
    assert_eq!(catalog.len(), 11);
}

#[test]
fn register_rejects_current_version_not_in_list() {
    let mut catalog = SectorCatalog::new();
    // Valid semver, but not one of the declared schema_versions.
    let descriptor = provisional_descriptor("2.0.0", vec!["1.0.0".into()]);
    assert!(matches!(
        catalog.register(descriptor),
        Err(CatalogError::CurrentVersionNotListed { .. })
    ));
    assert_eq!(catalog.len(), 11);
}

/// Parity guard: the closed [`Sector`](crate::domain::sector::Sector) enum
/// and the open [`SectorCatalog`] must describe the same set of
/// *compile-time* sectors. Runtime-registered sectors degrade to
/// `SectorData::Other`, but every typed `Sector` variant (except `Other`)
/// must have an embedded catalog entry, and the embedded catalog must not
/// carry a key with no corresponding variant. This stops the "four spellings
/// of a sector" drift from reappearing across the enum ↔ catalog boundary.
#[test]
fn sector_enum_and_catalog_agree() {
    use crate::domain::sector::Sector;

    let catalog = SectorCatalog::new();

    // Every typed Sector variant (except Other) must be in the catalog.
    let typed = [
        Sector::Battery,
        Sector::Textile,
        Sector::UnsoldGoods,
        Sector::Steel,
        Sector::Electronics,
        Sector::Construction,
        Sector::Tyre,
        Sector::Toy,
        Sector::Aluminium,
        Sector::Furniture,
        Sector::Detergent,
    ];
    for sector in &typed {
        let key = sector.catalog_key();
        assert!(
            catalog.get(key).is_some(),
            "Sector::{sector:?} (key '{key}') has no embedded catalog entry"
        );
    }

    // No catalog entry without a typed Sector variant.
    let typed_keys: std::collections::HashSet<&str> =
        typed.iter().map(Sector::catalog_key).collect();
    for key in catalog.keys() {
        assert!(
            typed_keys.contains(key),
            "catalog key '{key}' has no corresponding typed Sector variant"
        );
    }
}

/// Drift guard: [`crate::domain::sector::Sector::minimum_retention_years`] is
/// a compile-time constant kept on the domain enum for wasm32/no-catalog
/// callers; the catalog's own `retention_years` is the value production code
/// actually applies at publish time (see `Passport.retention_until`'s doc
/// comment). Nothing else ties the two together — this is what stops them
/// from silently diverging if a future delegated act sets a sector-specific
/// retention floor.
#[test]
fn retention_years_matches_sector_enum() {
    use crate::domain::sector::Sector;

    let catalog = SectorCatalog::new();
    let typed = [
        Sector::Battery,
        Sector::Textile,
        Sector::UnsoldGoods,
        Sector::Steel,
        Sector::Electronics,
        Sector::Construction,
        Sector::Tyre,
        Sector::Toy,
        Sector::Aluminium,
        Sector::Furniture,
        Sector::Detergent,
    ];
    for sector in &typed {
        let key = sector.catalog_key();
        let descriptor = catalog
            .get(key)
            .unwrap_or_else(|| panic!("Sector::{sector:?} (key '{key}') has no catalog entry"));
        assert_eq!(
            sector.minimum_retention_years(),
            descriptor.retention_years,
            "Sector::{sector:?}.minimum_retention_years() disagrees with catalog '{key}'.retention_years"
        );
    }
}

/// Compliance citation (domain Gap / watchlist 🔴): the ESPR unsold-goods
/// destruction ban is **Article 25 / Annex VII**, not Article 22. A wrong
/// citation in a compliance artifact erodes auditor trust.
/// Source: Regulation (EU) 2024/1781 (ESPR) Article 25, Annex VII.
#[test]
fn unsold_goods_cites_espr_article_25() {
    let catalog = SectorCatalog::new();
    let textile = catalog.get("unsold-goods").expect("unsold-goods present");
    let basis = textile.legal_basis.join(" ");
    assert!(
        basis.contains("Article 25"),
        "unsold-goods legal basis must cite ESPR Article 25, got: {basis}"
    );
    assert!(
        !basis.contains("Article 22"),
        "the incorrect Article 22 citation must be gone, got: {basis}"
    );
}

#[test]
fn descriptor_round_trips_camel_case() {
    let catalog = SectorCatalog::new();
    let battery = catalog.get("battery").unwrap();
    let json = serde_json::to_value(battery).unwrap();
    assert_eq!(json["dppAppliesFrom"], "2027-02-18");
    assert_eq!(json["status"], "in_force");
    let back: SectorDescriptor = serde_json::from_value(json).unwrap();
    assert_eq!(back.key, "battery");
}

// Drift guard: every key in a sector's access_tiers manifest must correspond to
// a real JSON field in that sector's current schema. A key that doesn't match any
// schema property silently fails to gate any field — the redaction is a no-op.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn access_tiers_keys_match_schema_properties() {
    use crate::schemas::VersionedSchemaRegistry;
    let catalog = SectorCatalog::new();
    let registry = VersionedSchemaRegistry::new();

    for descriptor in catalog.all() {
        if descriptor.access_tiers.is_empty() {
            continue;
        }
        let version: semver::Version =
            descriptor
                .current_schema_version
                .parse()
                .unwrap_or_else(|_| {
                    panic!(
                        "sector '{}' currentSchemaVersion '{}' is not valid semver",
                        descriptor.key, descriptor.current_schema_version
                    )
                });
        let schema_json = registry.get(&descriptor.key, &version).unwrap_or_else(|| {
            panic!(
                "schema not found for sector '{}' v{}",
                descriptor.key, descriptor.current_schema_version
            )
        });
        let schema: serde_json::Value =
            serde_json::from_str(schema_json).expect("embedded schema must be valid JSON");
        let properties = schema
            .get("properties")
            .and_then(|p| p.as_object())
            .unwrap_or_else(|| {
                panic!(
                    "schema for sector '{}' has no top-level 'properties' object",
                    descriptor.key
                )
            });

        for key in descriptor.access_tiers.keys() {
            assert!(
                properties.contains_key(key),
                "access_tiers key '{}' in sector '{}' does not match any property in schema v{} \
                 (properties: {:?}). Either rename the key to match the serialised field name, \
                 or remove it — a mismatched key silently fails to gate the field.",
                key,
                descriptor.key,
                descriptor.current_schema_version,
                properties.keys().collect::<Vec<_>>()
            );
        }
    }
}

// The key enforcement: catalog ↔ schema registry must agree, so the
// "four spellings of a sector" problem cannot silently reappear.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn catalog_agrees_with_schema_registry() {
    use crate::schemas::VersionedSchemaRegistry;
    let catalog = SectorCatalog::new();
    let registry = VersionedSchemaRegistry::new();

    // Every schema version a sector declares must exist in the registry,
    // and its current version must be one of them.
    for d in catalog.all() {
        let reg_versions: Vec<String> = registry
            .versions_for(&d.key)
            .iter()
            .map(|v| v.to_string())
            .collect();
        for v in &d.schema_versions {
            assert!(
                reg_versions.contains(v),
                "catalog sector '{}' declares schema {v} not embedded in the registry (registry has {reg_versions:?})",
                d.key
            );
        }
        assert!(
            d.schema_versions.contains(&d.current_schema_version),
            "catalog sector '{}' currentSchemaVersion {} is not in its schemaVersions {:?}",
            d.key,
            d.current_schema_version,
            d.schema_versions
        );
    }

    // No orphan schemas: every registry sector must have a catalog entry.
    for sector in registry.sectors() {
        assert!(
            catalog.get(sector).is_some(),
            "schema registry has sector '{sector}' with no catalog entry"
        );
    }

    // Every in-force sector must declare a plugin binding.
    for d in catalog.in_force() {
        assert!(
            d.plugin.is_some(),
            "in-force sector '{}' must declare a plugin binding",
            d.key
        );
    }
}
