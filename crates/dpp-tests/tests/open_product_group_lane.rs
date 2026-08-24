//! The product group axis is open: a product group this build has no typed variant
//! for still round-trips, keyed by its own name.
//!
//! Before this, `ProductGroup` and `ProductGroupData` were plain derives — `ProductGroupData`
//! internally tagged on `product_group` — so an unrecognised tag failed to
//! deserialize outright, and the `Other` variant matched only the literal
//! string `"other"`, discarding the real key. The catalog, the schema registry
//! and the plugin manifests were all string-keyed data, and the wire was the
//! one closed part: adding a product group meant releasing this crate.
//!
//! These tests pin the property that makes it data instead.

use dpp_domain::{ProductGroup, ProductGroupData};
use serde_json::json;

/// The tag of a product group deliberately absent from this build's catalog.
const UNKNOWN: &str = "packaging";

#[test]
fn an_unknown_product_group_tag_deserializes_instead_of_failing() {
    let wire = json!({
        "productGroup": UNKNOWN,
        "materialFamily": "corrugated-fibreboard",
        "recycledContentPct": 82.5,
    });

    let data: ProductGroupData = serde_json::from_value(wire)
        .expect("an unknown product_group must not be a deserialization error");

    match &data {
        ProductGroupData::Other { product_group, .. } => assert_eq!(product_group, UNKNOWN),
        other => panic!("expected an untyped payload, got {other:?}"),
    }
}

#[test]
fn an_unknown_product_group_keeps_its_identity_through_a_round_trip() {
    let wire = json!({
        "productGroup": UNKNOWN,
        "materialFamily": "corrugated-fibreboard",
        "recycledContentPct": 82.5,
    });

    let data: ProductGroupData = serde_json::from_value(wire.clone()).expect("deserializes");

    // Compare serialised bytes, not `Value`s: `Value` equality is
    // order-insensitive, so it would pass even if the round trip reordered or
    // renormalised the object. A signature is computed over bytes, so bytes are
    // what has to survive.
    assert_eq!(
        serde_json::to_string(&data).expect("serializes"),
        serde_json::to_string(&wire).expect("serializes"),
        "an untyped product_group payload was altered by a round trip"
    );
}

#[test]
fn the_product_group_discriminant_carries_the_unknown_key() {
    let data: ProductGroupData =
        serde_json::from_value(json!({ "productGroup": UNKNOWN })).expect("deserializes");

    // `product_group()` must report the real key, not a placeholder — the catalog,
    // schema registry and plugin host all dispatch on it.
    let product_group = data.product_group();
    assert_eq!(product_group.catalog_key(), UNKNOWN);
    assert_eq!(product_group, ProductGroup::Other(UNKNOWN.to_owned()));
}

#[test]
fn the_product_group_enum_round_trips_an_unknown_tag() {
    let product_group: ProductGroup = serde_json::from_value(json!(UNKNOWN)).expect("deserializes");
    assert_eq!(product_group, ProductGroup::Other(UNKNOWN.to_owned()));
    assert_eq!(product_group.wire_str(), UNKNOWN);
    assert_eq!(
        serde_json::to_value(&product_group).unwrap(),
        json!(UNKNOWN)
    );
}

#[test]
fn two_unknown_product_groups_stay_distinct() {
    // The old `Other` collapsed every unrecognised product group to one value, so two
    // different product groups became indistinguishable after a round trip.
    let a: ProductGroupData =
        serde_json::from_value(json!({ "productGroup": "packaging" })).unwrap();
    let b: ProductGroupData =
        serde_json::from_value(json!({ "productGroup": "automotive" })).unwrap();

    assert_ne!(a.product_group(), b.product_group());
    assert_eq!(a.product_group().catalog_key(), "packaging");
    assert_eq!(b.product_group().catalog_key(), "automotive");
}

#[test]
fn known_product_groups_still_deserialize_to_their_typed_variants() {
    // The open lane must not cost the typed lane: an in-force product group still
    // lands on its struct, not in the untyped fallback.
    let battery = json!({
        "productGroup": "battery",
        "gtin": "09506000134352",
        "batteryChemistry": "lfp",
        "nominalVoltageV": 3.2,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "batteryType": "portable",
    });

    let data: ProductGroupData = serde_json::from_value(battery).expect("battery deserializes");
    assert!(
        matches!(data, ProductGroupData::Battery(_)),
        "a known product_group fell through to the untyped variant: {data:?}"
    );
    assert_eq!(data.product_group(), ProductGroup::Battery);
}

#[test]
fn the_kebab_case_catalog_spelling_resolves_to_the_typed_variant() {
    // Two spellings are in circulation — the catalog's kebab-case key and the
    // camelCase wire tag. Both name one product group, and neither may fall
    // through to the untyped lane.
    assert_eq!(
        ProductGroup::from_wire_tag("unsold-goods"),
        ProductGroup::UnsoldGoods
    );
    assert_eq!(
        ProductGroup::from_wire_tag("unsoldGoods"),
        ProductGroup::UnsoldGoods
    );
}

#[test]
fn a_payload_with_no_product_group_tag_is_still_an_error() {
    // Opening the lane must not open it so far that an untagged payload is
    // accepted: with no tag there is no identity to preserve.
    let result: Result<ProductGroupData, _> = serde_json::from_value(json!({ "someField": 1 }));
    assert!(
        result.is_err(),
        "a payload with no product_group tag must be refused"
    );
}

#[test]
fn the_untyped_variant_cannot_alias_a_typed_product_group() {
    // `Other` holding a typed product group's tag would be a second representation of
    // that product group which does not compare equal to the first: it would miss
    // every typed match arm and be refused by validation, while the same bytes
    // deserialized normally produce a valid typed value.
    for typed in ["battery", "textile", "unsoldGoods", "unsold-goods", "toy"] {
        assert!(
            ProductGroupData::other(json!({ "productGroup": typed })).is_none(),
            "ProductGroupData::other accepted '{typed}', which has a typed variant"
        );
    }

    // Unknown tags, and payloads carrying none, are still constructible.
    assert!(ProductGroupData::other(json!({ "productGroup": UNKNOWN })).is_some());
    assert!(ProductGroupData::other(json!({ "someField": 1 })).is_some());
}
