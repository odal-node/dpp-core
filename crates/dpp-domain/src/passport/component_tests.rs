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

/// The **old** element shape still parses, and arrives with no qualifiers.
///
/// This is not leniency for its own sake. `componentRefs` is part of the signed
/// public view that other operators' nodes fetch to verify a bill of materials.
/// Those passports are signed and belong to someone else, so they can never be
/// rewritten into the new shape — and a reader that refuses them refuses data
/// that is correct, current and unforgeable.
///
/// Refusing would also not fail quietly. A verification walk that cannot parse
/// an entry reports it as a malformed reference, which is graded as an integrity
/// violation alongside a hash mismatch — so an upgraded node would accuse a
/// not-yet-upgraded node of tampering purely from a version difference.
#[test]
fn the_element_shape_this_replaced_still_parses() {
    let bare = serde_json::json!({
        "uri": "https://example.test/dpp/cell",
        "publicJwsHash": "b".repeat(64),
    });

    let edge: ComponentRef =
        serde_json::from_value(bare).expect("the pre-Phase-3 element shape must still parse");

    assert_eq!(edge.reference, reference());
    assert_eq!(
        edge.quantity, None,
        "an edge written before qualifiers existed declares none"
    );
    assert_eq!(edge.role, None);
}

/// The same, reached through a whole passport rather than one element.
///
/// The walk that matters reads `componentRefs` off a fetched passport, so the
/// tolerance has to survive the container, not just the element in isolation.
#[test]
fn a_passport_carrying_the_old_element_shape_still_reads() {
    let passport = crate::test_support::sample_passport();
    let mut doc = serde_json::to_value(&passport).expect("serialise");
    doc.as_object_mut().expect("object").insert(
        "componentRefs".to_owned(),
        serde_json::json!([{
            "uri": "https://example.test/dpp/cell",
            "publicJwsHash": "b".repeat(64),
        }]),
    );

    let back: Passport =
        serde_json::from_value(doc).expect("a passport with pre-Phase-3 edges must still read");

    assert_eq!(back.component_refs.len(), 1);
    assert_eq!(back.component_refs[0].reference, reference());
    assert_eq!(back.component_refs[0].quantity, None);
}

/// Tolerance is read-only: the current shape is the only one ever written.
///
/// Accepting the old shape must not turn into emitting it — a round-trip has to
/// normalise, or the old shape would propagate forward indefinitely.
#[test]
fn the_old_shape_is_read_but_never_written_back() {
    let bare = serde_json::json!({
        "uri": "https://example.test/dpp/cell",
        "publicJwsHash": "b".repeat(64),
    });

    let edge: ComponentRef = serde_json::from_value(bare).expect("parses");
    let out = serde_json::to_value(&edge).expect("serialises");

    assert!(
        out.get("reference").is_some(),
        "a re-serialised edge must use the current shape"
    );
    assert!(
        out.get("uri").is_none(),
        "the old shape must not survive a round trip"
    );
}

/// A genuinely malformed entry is still refused.
///
/// The tolerant reader accepts exactly two shapes. Something that is neither —
/// a reference missing its pin, say — must still fail, or the walk would treat
/// nonsense as a valid edge.
#[test]
fn an_entry_matching_neither_shape_is_still_refused() {
    let nonsense = serde_json::json!({ "uri": "https://example.test/dpp/cell" });

    serde_json::from_value::<ComponentRef>(nonsense)
        .expect_err("an entry that is neither shape must not parse");
}
