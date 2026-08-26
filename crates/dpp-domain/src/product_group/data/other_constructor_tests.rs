//! The `Other` variant constructor for runtime-registered groups.

use super::*;
use serde_json::json;

use crate::product_group::ProductGroup;

/// A non-object payload is refused rather than accepted untagged.
///
/// The hazard is not the round-trip failure, which is merely wrong. It is
/// that `Serialize` only stamps the `product_group` tag onto an object, so an
/// untagged payload slips past `dpp-aas`'s unknown-product group backstop — which
/// keys off exactly that tag — and is filtered by a policy defaulting to
/// `Public`.
#[test]
fn a_non_object_payload_is_refused() {
    for payload in [
        json!([{ "secret": "value" }]),
        json!(42),
        json!("battery"),
        json!(null),
        json!(true),
    ] {
        assert!(
            ProductGroupData::other(payload.clone()).is_none(),
            "a non-object payload must not become ProductGroupData::Other: {payload}"
        );
    }
}

/// An object for an untyped product group is still accepted, tag or not.
#[test]
fn an_object_for_an_unknown_product_group_is_still_accepted() {
    let tagged = ProductGroupData::other(json!({ "productGroup": "quantum-widget", "spinPct": 3 }))
        .expect("an unknown tagged product_group is representable");
    assert_eq!(
        tagged.product_group(),
        ProductGroup::Other("quantum-widget".into())
    );

    let untagged = ProductGroupData::other(json!({ "spinPct": 3 }))
        .expect("an untagged object defaults to other");
    assert_eq!(
        untagged.product_group(),
        ProductGroup::Other("other".into())
    );
}

/// A typed product group's tag is still refused — the pre-existing rule.
#[test]
fn a_typed_product_groups_tag_is_still_refused() {
    assert!(ProductGroupData::other(json!({ "productGroup": "battery" })).is_none());
}

/// Everything this constructor accepts round-trips.
///
/// The property the object guard buys: previously a caller could build a
/// value that serialised to something `Deserialize` then rejected.
#[test]
fn everything_accepted_round_trips() {
    for payload in [
        json!({ "productGroup": "quantum-widget", "spinPct": 3 }),
        json!({ "spinPct": 3 }),
        json!({}),
    ] {
        let Some(built) = ProductGroupData::other(payload.clone()) else {
            continue;
        };
        let wire = serde_json::to_value(&built).expect("serialises");
        assert!(
            wire.get("productGroup")
                .and_then(serde_json::Value::as_str)
                .is_some(),
            "the serialised form must carry a product_group tag: {wire}"
        );
        let back: ProductGroupData = serde_json::from_value(wire).expect("round-trips");
        assert_eq!(back, built);
    }
}
