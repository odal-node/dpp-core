//! `ProductGroupCatalog` load, gating, registration, and cross-artifact parity tests.

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

/// Parity guard: the closed [`ProductGroup`](crate::domain::product_group::ProductGroup) enum
/// and the open [`ProductGroupCatalog`] must describe the same set of
/// *compile-time* product groups. Runtime-registered product groups degrade to
/// `ProductGroupData::Other`, but every typed `ProductGroup` variant (except `Other`)
/// must have an embedded catalog entry, and the embedded catalog must not
/// carry a key with no corresponding variant. This stops the "four spellings
/// of a product group" drift from reappearing across the enum ↔ catalog boundary.
#[test]
fn product_group_enum_and_catalog_agree() {
    use crate::domain::product_group::ProductGroup;

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

// ── The instrument axis ──────────────────────────────────────────────────────

/// Guards the assumption that ESPR is the only source of a DPP obligation.
/// Battery, toy, detergent, construction and electronics each derive from their
/// own act; if this ever collapses to all-ESPR, something has been flattened.
#[test]
fn most_product_groups_are_not_reached_by_espr() {
    let instruments = InstrumentCatalog::new();
    let non_espr = ProductGroupCatalog::new()
        .all()
        .iter()
        .filter(|d| {
            instruments
                .bindings_for(&d.key)
                .iter()
                .all(|(i, _)| i.id != "espr")
        })
        .count();
    assert_eq!(
        non_espr, 5,
        "expected 5 product groups reached by something other than ESPR \
         (battery, toy, detergent, construction, electronics)"
    );
}

/// The two axes must stay orthogonal: a binding that is not in force gates
/// identically whatever kind of act it comes from. If this fails, the kind of
/// instrument has leaked into the determination path.
#[test]
fn instrument_kind_does_not_affect_determination_gating() {
    for instrument in InstrumentCatalog::new().all() {
        for binding in &instrument.product_groups {
            if binding.status == RegulatoryStatus::InForce {
                continue;
            }
            assert!(
                !binding.allows_determination(),
                "'{}' under '{}' (kind {:?}) must not allow determinations",
                binding.product_group,
                instrument.id,
                instrument.kind
            );
        }
    }
}

// ── Watch status ─────────────────────────────────────────────────────────────

#[test]
fn watch_never_allows_determination() {
    assert!(!RegulatoryStatus::Watch.allows_determination());
}

#[test]
fn in_force_with_future_passport_date_still_determines() {
    // Regression guard. `dppAppliesFrom` is the passport-obligation date and is
    // NOT the determination gate. Battery's passport is required from
    // 2027-02-18, but its Art. 9 mercury/cadmium prohibitions have applied
    // since 2008 and are determinable today. Gating determinations on the
    // passport date would suppress a legally valid non-compliance finding.
    let catalog = InstrumentCatalog::new();
    let due = catalog.passport_due_for("battery").expect("a fixed date");
    assert_eq!(due.date, "2027-02-18");
    let determinable = catalog.determinable_for("battery");
    assert_eq!(determinable.len(), 1);
    assert_eq!(determinable[0].0.id, "battery-reg-2023-1542");
}

#[test]
fn every_manifest_round_trips() {
    for d in ProductGroupCatalog::new().all().iter() {
        let json = serde_json::to_string(d).expect("serialise");
        let back: ProductGroupDescriptor = serde_json::from_str(&json).expect("deserialise");
        // ProductGroupDescriptor is not PartialEq, and `disclosure` is a HashMap whose
        // serialised key order is not stable — compare as Value, which is
        // order-insensitive for maps.
        assert_eq!(
            serde_json::to_value(&back).expect("re-serialise"),
            serde_json::to_value(d).expect("serialise"),
            "round-trip changed product_group '{}'",
            d.key
        );
    }
}

/// ProductGroups whose catalog `productCategories` mirror a schema enum, and the
/// property that enumerates them.
///
/// The correspondence is **not derivable** from the data — depending on product group
/// the categories live under `productCategory`, `productType`, `batteryType`,
/// `productFamily`, `productionRoute` or `tyreClass` — so it is declared here.
/// A product group absent from this table is simply not cross-checked; `textile`, for
/// example, declares categories but its schema has no enum for them.
const CATEGORY_ENUM_PROPERTY: &[(&str, &str)] = &[
    ("aluminium", "productionRoute"),
    ("battery", "batteryType"),
    ("construction", "productFamily"),
    ("detergent", "productType"),
    ("electronics", "productCategory"),
    ("furniture", "productType"),
    ("steel", "productCategory"),
    ("tyre", "tyreClass"),
    ("unsold-goods", "productCategory"),
];

/// Drift guard: a catalog product category that is not a legal value of the
/// corresponding schema enum is a value nothing can ever validate against.
///
/// This existed as two spellings of one concept — the catalog said `sli` and
/// `clothing_accessories` where the schemas said `starting-lighting-ignition`
/// and `accessories`. Neither was load-bearing, because
/// `ProductGroupDescriptor::product_categories` has no reader in Rust today; both
/// would have become load-bearing the moment one appeared.
#[test]
fn product_categories_are_legal_values_of_their_schema_enum() {
    use crate::schemas::VersionedSchemaRegistry;

    let catalog = ProductGroupCatalog::new();
    let registry = VersionedSchemaRegistry::new();

    for (product_group_key, property) in CATEGORY_ENUM_PROPERTY {
        let descriptor = catalog.get(product_group_key).unwrap_or_else(|| {
            panic!("product_group '{product_group_key}' is in the table but not the catalog")
        });
        let version: semver::Version = descriptor
            .current_schema_version
            .parse()
            .expect("currentSchemaVersion is valid semver");
        let schema_json = registry
            .get(product_group_key, &version)
            .unwrap_or_else(|| panic!("no schema for '{product_group_key}' v{version}"));
        let schema: serde_json::Value =
            serde_json::from_str(schema_json).expect("schema is valid JSON");

        let allowed: Vec<&str> = schema["properties"][property]["enum"]
            .as_array()
            .unwrap_or_else(|| {
                panic!("'{product_group_key}' schema property '{property}' has no enum — stale table row")
            })
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect();

        for category in &descriptor.product_categories {
            assert!(
                allowed.contains(&category.as_str()),
                "product_group '{product_group_key}' lists product category '{category}', which is not a legal \
                 value of schema property '{property}' ({allowed:?})"
            );
        }
    }
}
