//! The instrument axis: which acts reach a product group, and how watch status
//! and instrument kind bear on determination.

use super::*;
use crate::catalog::{ProductGroupCatalog, ProductGroupDescriptor, RegulatoryStatus};

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
    // `unsold-goods` is deliberately absent. Impl. Reg. (EU) 2026/2 Art. 3
    // delimits a disclosure by CN code, not by a category name, so v2.0.0 has
    // no category enum for a catalog row to be checked against — and the
    // descriptor's `productCategories` is empty for the same reason.
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
