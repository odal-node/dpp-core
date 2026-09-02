//! `ComponentRef` carries its qualifiers, and the old element shape is refused.

use super::component::{ComponentRef, Quantity};
use super::record::Passport;
use super::reference::PassportRef;

fn reference() -> PassportRef {
    PassportRef {
        uri: "https://example.test/dpp/cell".into(),
        public_jws_hash: "b".repeat(64),
    }
}

/// A fully-qualified edge round-trips, and nests rather than flattening.
#[test]
fn component_ref_round_trips_with_both_qualifiers() {
    let edge = ComponentRef {
        reference: reference(),
        quantity: Some(Quantity {
            value: 1.4,
            unit: Some("kg".into()),
        }),
        role: Some("outer shell".into()),
    };

    let json = serde_json::to_value(&edge).expect("edge serialises");
    assert_eq!(json["reference"]["publicJwsHash"], "b".repeat(64));
    assert_eq!(json["quantity"]["value"], 1.4);
    assert_eq!(json["quantity"]["unit"], "kg");
    assert_eq!(json["role"], "outer shell");

    let back: ComponentRef = serde_json::from_value(json).expect("edge deserialises");
    assert_eq!(back, edge);
}

/// Both qualifiers are optional, and an absent one is omitted rather than null.
///
/// A BOM whose granularity no delegated act defines is the normal case, not a
/// degenerate one — most edges will carry neither qualifier.
#[test]
fn a_bare_component_ref_omits_its_qualifiers() {
    let edge = ComponentRef {
        reference: reference(),
        quantity: None,
        role: None,
    };

    let json = serde_json::to_value(&edge).expect("edge serialises");
    let object = json.as_object().expect("object");
    assert!(
        !object.contains_key("quantity"),
        "an absent quantity must be omitted, not serialised as null"
    );
    assert!(
        !object.contains_key("role"),
        "an absent role must be omitted, not serialised as null"
    );

    let back: ComponentRef = serde_json::from_value(json).expect("edge deserialises");
    assert_eq!(back, edge);
}

/// A dimensionless count is a quantity with no unit, not a missing quantity.
#[test]
fn a_count_is_a_quantity_without_a_unit() {
    let quantity = Quantity {
        value: 2.0,
        unit: None,
    };

    let json = serde_json::to_value(&quantity).expect("quantity serialises");
    assert_eq!(json["value"], 2.0);
    assert!(
        !json.as_object().expect("object").contains_key("unit"),
        "a dimensionless count omits the unit rather than inventing one"
    );
}

/// A document carrying the **old** `componentRefs` element shape is refused.
///
/// Unlike the `parentPassportRef` → `derivedFrom` rename, this change needs no
/// entry in `REMOVED_ENVELOPE_KEYS`, because the key name is unchanged and
/// `#[serde(default)]` applies only when a key is *absent*. A present key with
/// wrong-shaped elements therefore fails on its own.
///
/// That is worth pinning rather than assuming: "it happens to error today" and
/// "it is guaranteed to error" are different claims, and only the second one
/// survives someone later adding a `#[serde(default)]` or a lenient
/// deserializer to `ComponentRef`.
#[test]
fn the_old_component_ref_element_shape_is_refused() {
    let passport = crate::test_support::sample_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc.as_object_mut().expect("object").insert(
        "componentRefs".to_owned(),
        serde_json::json!([{
            "uri": "https://example.test/dpp/cell",
            "publicJwsHash": "b".repeat(64),
        }]),
    );

    let err = serde_json::from_value::<Passport>(doc)
        .expect_err("the pre-Phase-3 element shape must not deserialize");
    let message = err.to_string();
    assert!(
        message.contains("reference"),
        "the error should name the field the old shape lacks, got: {message}"
    );
}
