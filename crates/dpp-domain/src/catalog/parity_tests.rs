//! Parity guards: the catalog, the closed `ProductGroup` enum, the schema
//! registry and the access policy must all describe the same product groups.

use super::*;
use crate::instrument::InstrumentCatalog;

/// Parity guard: the closed [`ProductGroup`](crate::product_group::ProductGroup) enum
/// and the open [`ProductGroupCatalog`] must describe the same set of
/// *compile-time* product groups. Runtime-registered product groups degrade to
/// `ProductGroupData::Other`, but every typed `ProductGroup` variant (except `Other`)
/// must have an embedded catalog entry, and the embedded catalog must not
/// carry a key with no corresponding variant. This stops the "four spellings
/// of a product group" drift from reappearing across the enum ↔ catalog boundary.
#[test]
fn product_group_enum_and_catalog_agree() {
    use crate::product_group::ProductGroup;

    let catalog = ProductGroupCatalog::new();

    // Every typed ProductGroup variant (except Other) must be in the catalog.
    let typed = [
        ProductGroup::Battery,
        ProductGroup::Textile,
        ProductGroup::UnsoldGoods,
        ProductGroup::Steel,
        ProductGroup::Electronics,
        ProductGroup::Construction,
        ProductGroup::Tyre,
        ProductGroup::Toy,
        ProductGroup::Aluminium,
        ProductGroup::Furniture,
        ProductGroup::Mattress,
        ProductGroup::Detergent,
    ];
    for product_group in &typed {
        let key = product_group.catalog_key();
        assert!(
            catalog.get(key).is_some(),
            "ProductGroup::{product_group:?} (key '{key}') has no embedded catalog entry"
        );
    }

    // No catalog entry without a typed ProductGroup variant.
    let typed_keys: std::collections::HashSet<&str> =
        typed.iter().map(ProductGroup::catalog_key).collect();
    for key in catalog.keys() {
        assert!(
            typed_keys.contains(key),
            "catalog key '{key}' has no corresponding typed ProductGroup variant"
        );
    }
}

/// Every product group we ship has a retention period some act imposes.
///
/// Retention has moved twice now. It was once duplicated between a manifest
/// field and a hardcoded match on the `ProductGroup` enum, where the enum was what
/// production applied while the field was documented as authoritative. It then
/// sat on the descriptor alone, which assumed one act per product group. It now
/// sits on the binding, with the act that imposes it, and the catalog folds the
/// applicable set to a maximum.
///
/// What still needs guarding is that the figure is always *there*: a product
/// group that reaches publish with no retention period has no obligation the
/// code can enforce, and there is no safe default for the length of someone
/// else's legal duty.
#[test]
fn every_product_group_has_a_retention_period_from_some_act() {
    let catalog = ProductGroupCatalog::new();
    let instruments = InstrumentCatalog::new();
    for descriptor in catalog.all() {
        let key = descriptor.key.as_str();
        let (years, _basis) = instruments
            .retention_for(key)
            .unwrap_or_else(|| panic!("no act imposes a retention period on '{key}'"));
        assert!(
            years > 0,
            "product group '{key}' declares no retention period"
        );
    }

    // A product group no act reaches yields None, so callers must fail closed
    // rather than substitute a default for an unknown legal obligation.
    assert_eq!(instruments.retention_for("packaging"), None);
}

/// Compliance citation: the ESPR unsold-goods destruction ban is **Article 25 /
/// Annex VII**, not Article 22. A wrong citation in a compliance artifact erodes
/// auditor trust. The citation now lives on the binding that rests on it, so
/// this asks the instrument catalog rather than the product group.
/// Source: Regulation (EU) 2024/1781 (ESPR) Article 25, Annex VII.
#[test]
fn unsold_goods_cites_espr_article_25() {
    let instruments = InstrumentCatalog::new();
    let bindings = instruments.bindings_for("unsold-goods");
    assert!(
        !bindings.is_empty(),
        "unsold-goods must be reached by an act"
    );
    let basis: String = bindings
        .iter()
        .flat_map(|(_, b)| b.legal_basis.iter())
        .cloned()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(
        basis.contains("Art. 25"),
        "unsold-goods legal basis must cite ESPR Article 25, got: {basis}"
    );
    assert!(
        !basis.contains("Art. 22"),
        "the incorrect Article 22 citation must be gone, got: {basis}"
    );
}

#[test]
fn descriptor_round_trips_camel_case() {
    let catalog = ProductGroupCatalog::new();
    let battery = catalog.get("battery").unwrap();
    let json = serde_json::to_value(battery).unwrap();
    assert_eq!(json["currentSchemaVersion"], "2.6.0");
    // The law is not here and must not come back: a descriptor carrying its own
    // status or date is a descriptor asserting that one act governs it.
    for absent in ["status", "regime", "dppAppliesFrom", "retentionYears"] {
        assert!(
            json.get(absent).is_none(),
            "'{absent}' belongs on the instrument binding, not the product group"
        );
    }
    let back: ProductGroupDescriptor = serde_json::from_value(json).unwrap();
    assert_eq!(back.key, "battery");
}

// Drift guard: every key in a product group's disclosure manifest must correspond to
// a real JSON field in that product group's current schema. A key that doesn't match any
// schema property silently fails to gate any field — the redaction is a no-op.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn disclosure_keys_match_schema_properties() {
    use crate::schemas::VersionedSchemaRegistry;
    let catalog = ProductGroupCatalog::new();
    let registry = VersionedSchemaRegistry::new();

    for descriptor in catalog.all() {
        if descriptor.disclosure.is_empty() {
            continue;
        }
        let version: semver::Version =
            descriptor
                .current_schema_version
                .parse()
                .unwrap_or_else(|_| {
                    panic!(
                        "product_group '{}' currentSchemaVersion '{}' is not valid semver",
                        descriptor.key, descriptor.current_schema_version
                    )
                });
        let schema_json = registry.get(&descriptor.key, &version).unwrap_or_else(|| {
            panic!(
                "schema not found for product_group '{}' v{}",
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
                    "schema for product_group '{}' has no top-level 'properties' object",
                    descriptor.key
                )
            });

        for key in descriptor.disclosure.keys() {
            assert!(
                properties.contains_key(key),
                "disclosure key '{}' in product_group '{}' does not match any property in schema v{} \
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
// "four spellings of a product group" problem cannot silently reappear.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn catalog_agrees_with_schema_registry() {
    use crate::schemas::VersionedSchemaRegistry;
    let catalog = ProductGroupCatalog::new();
    let registry = VersionedSchemaRegistry::new();

    // Every schema version a product group declares must exist in the registry,
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
                "catalog product_group '{}' declares schema {v} not embedded in the registry (registry has {reg_versions:?})",
                d.key
            );
        }
        assert!(
            d.schema_versions.contains(&d.current_schema_version),
            "catalog product_group '{}' currentSchemaVersion {} is not in its schemaVersions {:?}",
            d.key,
            d.current_schema_version,
            d.schema_versions
        );
    }

    // No orphan schemas: every registry product group must have a catalog entry.
    for product_group in registry.product_groups() {
        assert!(
            catalog.get(product_group).is_some(),
            "schema registry has product_group '{product_group}' with no catalog entry"
        );
    }

    // Every product group with a binding act must declare a plugin binding.
    let instruments = InstrumentCatalog::new();
    for d in catalog.all() {
        if instruments.determinable_for(&d.key).is_empty() {
            continue;
        }
        assert!(
            d.plugin.is_some(),
            "product group '{}' has a binding act and must declare a plugin",
            d.key
        );
    }
}
