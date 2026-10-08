//! Array-element disclosures (RFC 9901 clause 4.2.2): the properties this module
//! guarantees beyond what the RFC's own examples pin.
//!
//! `rfc9901_array_tests.rs` runs the RFC's data. These are the properties that
//! data cannot reach: what an issuer's output looks like, which malformed tokens
//! are refused and which are read as something harmless, and that the whole
//! round trip agrees with a model written without any of this code.

use proptest::prelude::*;
use serde_json::{Value, json};

use super::tests::stub_jwt;
use super::{Disclosure, SdJwt, SdJwtError, build_payload, conceal_elements};

fn processed(payload: &Value, disclosures: Vec<Disclosure>) -> Result<Value, SdJwtError> {
    SdJwt::new(stub_jwt(payload), disclosures)
        .disclosed_payload()
        .map(Value::Object)
}

fn placeholder_of(d: &Disclosure) -> Value {
    json!({ "...": d.digest() })
}

#[test]
fn an_element_disclosure_encodes_the_pair_clause_4_2_2_defines() {
    use base64::Engine;
    let d = Disclosure::element_with_salt("salt".to_owned(), json!({ "a": [1, 2] }));
    let decoded = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(d.encoded())
        .expect("base64url");
    let array: Value = serde_json::from_slice(&decoded).expect("JSON");
    assert_eq!(array, json!(["salt", { "a": [1, 2] }]));

    let again = Disclosure::parse(d.encoded()).expect("reads back");
    assert_eq!(again, d);
    assert!(again.is_array_element());
    assert_eq!(again.claim_name(), None);
}

#[test]
fn an_element_can_be_any_json_value() {
    for value in [
        json!(null),
        json!(true),
        json!(7),
        json!(1.5),
        json!("s"),
        json!([1, [2]]),
        json!({ "k": "v" }),
    ] {
        let d = Disclosure::element(value.clone());
        let again = Disclosure::parse(d.encoded()).expect("reads back");
        assert_eq!(again.claim_value(), &value);
    }
}

#[test]
fn two_equal_elements_get_two_salts_and_so_two_digests() {
    let (kept, disclosures) = conceal_elements(&[json!("DE"), json!("DE")], |_, _| true);
    assert_eq!(disclosures.len(), 2);
    assert_ne!(disclosures[0].salt(), disclosures[1].salt());
    assert_ne!(disclosures[0].digest(), disclosures[1].digest());
    assert_eq!(kept[0], placeholder_of(&disclosures[0]));
    assert_eq!(kept[1], placeholder_of(&disclosures[1]));
}

#[test]
fn concealing_keeps_each_placeholder_in_the_position_it_replaced() {
    let items = [json!("a"), json!("b"), json!("c"), json!("d")];
    let (kept, disclosures) = conceal_elements(&items, |position, _| position % 2 == 1);

    assert_eq!(kept.len(), 4);
    assert_eq!(kept[0], json!("a"));
    assert_eq!(kept[2], json!("c"));
    // Not sorted: the second hidden element's digest need not follow the first's.
    assert_eq!(kept[1], placeholder_of(&disclosures[0]));
    assert_eq!(kept[3], placeholder_of(&disclosures[1]));
    assert_eq!(disclosures[0].claim_value(), &json!("b"));
    assert_eq!(disclosures[1].claim_value(), &json!("d"));
}

#[test]
fn the_predicate_is_given_each_position_and_value() {
    let items = [json!(1), json!("x"), json!(null)];
    let mut seen = Vec::new();
    let (kept, disclosures) = conceal_elements(&items, |position, value| {
        seen.push((position, value.clone()));
        false
    });
    assert_eq!(seen, vec![(0, json!(1)), (1, json!("x")), (2, json!(null))]);
    assert_eq!(kept, items);
    assert!(disclosures.is_empty());
}

#[test]
fn concealing_does_not_reach_into_an_element() {
    let items = [json!({ "inner": [1, 2] })];
    let (kept, disclosures) = conceal_elements(&items, |_, _| false);
    assert_eq!(kept, items);
    assert!(disclosures.is_empty());
}

#[test]
fn every_element_hidden_and_revealed_comes_back_in_order() {
    let items = vec![json!("x"), json!(2), json!({ "k": "v" }), json!([3])];
    let (kept, disclosures) = conceal_elements(&items, |_, _| true);
    let payload = build_payload(
        json!({ "list": kept })
            .as_object()
            .cloned()
            .expect("object"),
        true,
    );

    let shown = processed(&payload, disclosures).expect("processes");
    assert_eq!(shown["list"], Value::Array(items));
}

#[test]
fn a_placeholder_nobody_holds_a_disclosure_for_is_removed() {
    let payload = json!({ "list": ["kept", { "...": "bm90LWEtcmVhbC1kaWdlc3Q" }, "also"] });
    assert_eq!(
        processed(&payload, vec![]).expect("processes")["list"],
        json!(["kept", "also"])
    );
}

#[test]
fn an_array_of_nothing_but_placeholders_becomes_an_empty_array_not_a_missing_one() {
    let payload = json!({ "list": [{ "...": "YQ" }, { "...": "Yg" }] });
    assert_eq!(
        processed(&payload, vec![]).expect("processes")["list"],
        json!([])
    );
}

/// Clause 7.1 step 3.b.ii names one shape. An object with a second key, or whose
/// value is not a string, is an ordinary object that happens to carry a reserved
/// word, and it is left as it is.
#[test]
fn only_an_object_of_one_key_holding_a_string_is_a_placeholder() {
    let looks_close = [
        json!({ "...": "YQ", "other": 1 }),
        json!({ "...": 5 }),
        json!({ "...": null }),
        json!({ "...": ["YQ"] }),
        json!({ "..": "YQ" }),
        json!({}),
    ];
    let payload = json!({ "list": looks_close });
    assert_eq!(
        processed(&payload, vec![]).expect("processes")["list"],
        payload["list"]
    );
}

#[test]
fn a_revealed_element_is_read_for_digests_of_its_own() {
    // The element is an object that hides one member, and an array that hides one
    // element: clause 4.2.6, once through each door.
    let name = Disclosure::new("name", json!("Max")).expect("a property");
    let inner = Disclosure::element(json!("deep"));
    let as_object = Disclosure::element(json!({ "_sd": [name.digest()], "kept": 1 }));
    let as_array = Disclosure::element(json!(["a", { "...": inner.digest() }]));

    let payload = json!({
        "list": [{ "...": as_object.digest() }, { "...": as_array.digest() }]
    });
    let shown = processed(&payload, vec![as_object, as_array, name, inner]).expect("processes");

    assert_eq!(
        shown["list"],
        json!([{ "kept": 1, "name": "Max" }, ["a", "deep"]])
    );
}

#[test]
fn a_property_disclosure_cannot_stand_for_an_array_element() {
    let property = Disclosure::new("name", json!("Max")).expect("a property");
    let payload = json!({ "list": [{ "...": property.digest() }] });
    assert!(matches!(
        processed(&payload, vec![property.clone()]),
        Err(SdJwtError::WrongDisclosureKind(d)) if d == property.digest()
    ));
}

#[test]
fn an_element_disclosure_cannot_stand_for_an_object_property() {
    let element = Disclosure::element(json!("Max"));
    let payload = json!({ "_sd": [element.digest()], "_sd_alg": "sha-256" });
    assert!(matches!(
        processed(&payload, vec![element.clone()]),
        Err(SdJwtError::WrongDisclosureKind(d)) if d == element.digest()
    ));
}

#[test]
fn a_digest_may_not_stand_in_two_places_whichever_way_it_is_embedded() {
    let element = Disclosure::element(json!("x"));
    let twice_in_arrays =
        json!({ "a": [{ "...": element.digest() }], "b": [{ "...": element.digest() }] });
    assert!(matches!(
        processed(&twice_in_arrays, vec![element.clone()]),
        Err(SdJwtError::DuplicateDigest(d)) if d == element.digest()
    ));

    let twice_in_one = json!({ "a": [{ "...": element.digest() }, { "...": element.digest() }] });
    assert!(matches!(
        processed(&twice_in_one, vec![element.clone()]),
        Err(SdJwtError::DuplicateDigest(_))
    ));

    // The same digest in an `_sd` array and in an array. The mismatch of kinds
    // would also refuse it; the repetition is refused first, and neither is
    // read as a harmless collision.
    let across = json!({ "_sd": [element.digest()], "a": [{ "...": element.digest() }] });
    assert!(processed(&across, vec![element]).is_err());
}

#[test]
fn an_unmatched_placeholder_repeated_is_still_a_repeat() {
    // Nothing was disclosed, so there is nothing to collide with, and the token is
    // refused all the same: a repeated digest is a malformed token, not a claim.
    let payload = json!({ "list": [{ "...": "YQ" }, { "...": "YQ" }] });
    assert!(matches!(
        processed(&payload, vec![]),
        Err(SdJwtError::DuplicateDigest(d)) if d == "YQ"
    ));
}

#[test]
fn an_element_disclosure_no_digest_refers_to_refuses_the_token() {
    let stray = Disclosure::element(json!("x"));
    let payload = json!({ "list": ["kept"] });
    assert_eq!(
        processed(&payload, vec![stray]),
        Err(SdJwtError::UnusedDisclosures(1))
    );
}

#[test]
fn an_element_has_no_claim_name_to_look_for() {
    let (kept, disclosures) = conceal_elements(&[json!("x")], |_, _| true);
    let sd_jwt = SdJwt::new(stub_jwt(&json!({ "list": kept })), disclosures.clone());
    assert!(sd_jwt.digests_for_claim("x").is_empty());
    assert!(sd_jwt.digests_for_claim("...").is_empty());
    assert!(sd_jwt.digests_for_claim("").is_empty());

    // Chosen by digest, like any other.
    let presented = sd_jwt.present(&[&disclosures[0].digest()]);
    assert_eq!(presented.disclosures().len(), 1);
}

#[test]
fn a_placeholder_nested_in_arrays_and_objects_is_found() {
    let hidden = Disclosure::element(json!("found"));
    let payload = json!({
        "a": { "b": [[{ "c": [{ "...": hidden.digest() }] }]] }
    });
    let shown = processed(&payload, vec![hidden]).expect("processes");
    assert_eq!(shown["a"]["b"], json!([[{ "c": ["found"] }]]));
}

proptest! {
    /// Against a model written without any of this code: whatever subset of an
    /// array's elements is hidden, and whatever subset of those is revealed, what
    /// the Verifier is left with is the elements that were never hidden or were
    /// revealed, in their original order.
    #[test]
    fn the_verifier_is_left_with_exactly_the_visible_elements_in_order(
        elements in prop::collection::vec((any::<i64>(), any::<bool>(), any::<bool>()), 0..16)
    ) {
        let items: Vec<Value> = elements.iter().map(|(v, _, _)| json!(v)).collect();
        let hidden: Vec<bool> = elements.iter().map(|(_, h, _)| *h).collect();
        let revealed: Vec<bool> = elements.iter().map(|(_, _, r)| *r).collect();

        let (kept, disclosures) = conceal_elements(&items, |position, _| hidden[position]);
        let payload = build_payload(
            json!({ "list": kept }).as_object().cloned().expect("object"),
            !disclosures.is_empty(),
        );

        // The digests of the hidden elements, in array order, so `revealed` can be
        // read by position.
        let mut chosen = Vec::new();
        let mut next = 0;
        for (position, _) in items.iter().enumerate() {
            if hidden[position] {
                if revealed[position] {
                    chosen.push(disclosures[next].clone());
                }
                next += 1;
            }
        }

        let expected: Vec<Value> = items
            .iter()
            .enumerate()
            .filter(|(position, _)| !hidden[*position] || revealed[*position])
            .map(|(_, value)| value.clone())
            .collect();

        let shown = processed(&payload, chosen).expect("processes");
        prop_assert_eq!(&shown["list"], &Value::Array(expected));
    }

    /// Nothing the Verifier is given can make it panic. A random string is read
    /// as a Disclosure and as a whole token, and both answer `Ok` or `Err`.
    #[test]
    fn arbitrary_text_never_panics_the_reader(text in "\\PC{0,200}") {
        let _ = Disclosure::parse(&text);
        let _ = SdJwt::parse(&text).map(|sd| sd.disclosed_payload());
    }
}
