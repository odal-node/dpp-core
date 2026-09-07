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
/// `productFamily` or `tyreClass` — so it is declared here.
///
/// # Absence is no longer an exemption
///
/// This table used to say that a product group absent from it "is simply not
/// cross-checked", and named `textile` as the example. That made absence a
/// silent opt-out from the only guard on this axis, and textile spent that
/// exemption declaring three categories no schema defines anywhere.
/// `every_product_group_declaring_categories_is_cross_checked` closes it: a
/// group that declares categories must appear here, so the only way out is to
/// declare none.
///
/// # `aluminium` is not here, and that is the correction
///
/// It used to carry `("aluminium", "productionRoute")`. That row made this guard
/// certify a **production route** as a product category — the two are different
/// axes, and pointing a drift guard at the wrong property is worse than having
/// none, because it reports the axis as verified. The catalog no longer declares
/// aluminium categories, so there is nothing left for a row to check.
///
/// `unsold-goods`, `mattress` and `toy` are absent for the plain reason: they
/// declare no categories. Impl. Reg. (EU) 2026/2 Art. 3 delimits an unsold-goods
/// disclosure by CN code rather than a category name, so its lines carry
/// `cnCategories` and its descriptor list is empty.
const CATEGORY_ENUM_PROPERTY: &[(&str, &str)] = &[
    ("battery", "batteryType"),
    ("construction", "productFamily"),
    ("detergent", "productType"),
    ("electronics", "productCategory"),
    ("furniture", "productType"),
    ("steel", "productCategory"),
    ("tyre", "tyreClass"),
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

/// Completeness guard: the direction the check above cannot run.
///
/// `product_categories_are_legal_values_of_their_schema_enum` iterates
/// `CATEGORY_ENUM_PROPERTY`, so a product group missing from that table is not
/// checked — and nothing said the table had to be complete. Absence was a
/// silent exemption, and it was spent: `textile` declared `apparel`, `footwear`
/// and `home_textile` while its schema carried no enum-valued property at any
/// version, so all three were values nothing could set and nothing could
/// validate.
///
/// The two directions catch different things and neither subsumes the other.
/// The first says a declared category is a legal value of the property it is
/// checked against. This one says a group that declares categories is checked
/// **at all** — which is the assertion that would have made textile's three
/// values visible, and the one whose absence is the same defect as #224.
#[test]
fn every_product_group_declaring_categories_is_cross_checked() {
    let catalog = ProductGroupCatalog::new();

    let unchecked: Vec<&str> = catalog
        .all()
        .iter()
        .filter(|d| !d.product_categories.is_empty())
        .filter(|d| !CATEGORY_ENUM_PROPERTY.iter().any(|(key, _)| *key == d.key))
        .map(|d| d.key.as_str())
        .collect();

    assert!(
        unchecked.is_empty(),
        "these product groups declare `productCategories` and have no row in \
         CATEGORY_ENUM_PROPERTY, so nothing checks their values against a schema: \
         {unchecked:?} — either add a row naming the schema property that enumerates \
         them, or, if the group's act defines no category axis, declare none"
    );
}
