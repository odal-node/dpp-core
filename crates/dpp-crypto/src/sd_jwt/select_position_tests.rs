//! Where an array element is (`select.rs`): its position as the Issuer made the
//! array, what counts toward it, and what does not.

use std::collections::BTreeSet;

use serde_json::{Value, json};

use super::select_tests::{chosen, digest_named, fixture, object, set};
use super::tests::stub_jwt;
use super::{ClaimPath, Disclosure, SdJwt, SdJwtError, build_payload, conceal, conceal_elements};

// --- where an array element is ------------------------------------------------

#[test]
fn an_array_element_is_found_at_its_position_as_the_issuer_made_the_array() {
    let f = fixture();
    // list = [a*, b, c*, d*]. Position 2 is `c`, though `b` is the only element a
    // Verifier would see, and so would be the one at position 0.
    let digests = f
        .token
        .digests_for_paths(&[ClaimPath::claim("list").index(2)])
        .expect("in the token");
    assert_eq!(digests, vec![f.c.clone()]);

    let shown = f
        .token
        .present(&[f.c.as_str()])
        .expect("presentable")
        .disclosed_payload()
        .expect("processes");
    assert_eq!(shown["list"], json!(["b", "c"]));
}

#[test]
fn a_hidden_element_counts_toward_a_position_whether_or_not_it_is_revealed() {
    // [a*, b, c*, d]: asking for position 3 is `d`, in the clear, though two
    // elements before it are hidden and, in a presentation of this token, gone.
    let (items, disclosures) = conceal_elements(
        &[json!("a"), json!("b"), json!("c"), json!("d")],
        |position, _| position % 2 == 0,
    );
    let token = SdJwt::new(
        stub_jwt(&build_payload(object(json!({ "list": items })), false)),
        disclosures,
    );
    assert!(
        token
            .digests_for_paths(&[ClaimPath::claim("list").index(3)])
            .expect("in the token")
            .is_empty()
    );
    assert_eq!(
        token
            .digests_for_paths(&[ClaimPath::claim("list").index(2)])
            .expect("in the token")
            .len(),
        1
    );
}

#[test]
fn the_wildcard_chooses_every_element() {
    let f = fixture();
    let all = f
        .token
        .digests_for_paths(&[ClaimPath::claim("list").all_elements()])
        .expect("in the token");
    assert_eq!(
        all.iter().cloned().collect::<BTreeSet<_>>(),
        set([&f.a, &f.c, &f.d])
    );
    let shown = f
        .token
        .present(&all.iter().map(String::as_str).collect::<Vec<_>>())
        .expect("presentable")
        .disclosed_payload()
        .expect("processes");
    assert_eq!(shown["list"], json!(["a", "b", "c", "d"]));
}

#[test]
fn a_decoy_is_not_an_element() {
    // [a*, <decoy>, b*]: clause 4.2.5's padding, a digest that stands for nothing.
    let a = Disclosure::element(json!("a"));
    let b = Disclosure::element(json!("b"));
    let decoy = json!({ "...": "zzzz-a-digest-nothing-hashes-to" });
    let token = SdJwt::new(
        stub_jwt(&json!({
            "list": [
                { "...": a.digest() },
                decoy,
                { "...": b.digest() },
            ]
        })),
        vec![a.clone(), b.clone()],
    );

    assert_eq!(
        token
            .digests_for_paths(&[ClaimPath::claim("list").index(1)])
            .expect("in the token"),
        vec![b.digest()],
        "position 1 is `b`: the decoy between the two is not counted"
    );
    assert_eq!(
        token
            .digests_for_paths(&[ClaimPath::claim("list").index(2)])
            .unwrap_err(),
        SdJwtError::ClaimNotInToken(r#"["list",2]"#.to_owned()),
        "the array has two elements, whatever it was padded to"
    );
}

/// The limit stated in the module documentation, pinned so it is not mistaken for
/// more. A token already presented in part holds no Disclosure for the elements
/// that were withheld, and they look like decoys, so positions after them move.
#[test]
fn a_token_that_was_already_presented_in_part_counts_a_withheld_element_as_a_decoy() {
    let (items, disclosures) = conceal_elements(&[json!("a"), json!("b"), json!("c")], |_, _| true);
    let (a, b, c) = (
        disclosures[0].clone(),
        disclosures[1].clone(),
        disclosures[2].clone(),
    );
    let payload = stub_jwt(&json!({ "list": items }));

    let whole = SdJwt::new(payload.clone(), vec![a.clone(), b, c.clone()]);
    assert_eq!(
        whole
            .digests_for_paths(&[ClaimPath::claim("list").index(2)])
            .expect("in the token"),
        vec![c.digest()]
    );

    // The same credential after `b` was withheld: `c` is now at position 1.
    let in_part = SdJwt::new(payload, vec![a, c.clone()]);
    assert_eq!(
        in_part
            .digests_for_paths(&[ClaimPath::claim("list").index(1)])
            .expect("in the token"),
        vec![c.digest()]
    );
}

#[test]
fn one_member_of_one_element_of_a_hidden_array_of_objects_takes_only_what_leads_to_it() {
    let (first, first_d) = conceal(&object(json!({ "n": "A", "a": 1 })), |k| k == "a").unwrap();
    let (second, second_d) = conceal(&object(json!({ "n": "B", "a": 2 })), |k| k == "n").unwrap();
    let (people, element_d) =
        conceal_elements(&[Value::Object(first), Value::Object(second)], |_, _| true);
    let (root, root_d) = conceal(&object(json!({ "people": people })), |k| k == "people").unwrap();

    let people_digest = digest_named(&root_d, "people");
    let second_name = digest_named(&second_d, "n");
    let second_element = element_d[1].digest();
    let first_element = element_d[0].digest();
    let first_member = digest_named(&first_d, "a");

    let mut disclosures = root_d;
    disclosures.extend(element_d);
    disclosures.extend(first_d);
    disclosures.extend(second_d);
    let token = SdJwt::new(stub_jwt(&build_payload(root, true)), disclosures);

    let digests = chosen(&token, &[ClaimPath::claim("people").index(1).key("n")]);
    assert_eq!(
        digests,
        set([&people_digest, &second_element, &second_name]),
        "the array, the one element, and the one member"
    );
    // The first element, and everything in it, is not among them.
    assert!(!digests.contains(&first_element));
    assert!(!digests.contains(&first_member));

    let shown = token
        .present(&digests.iter().map(String::as_str).collect::<Vec<_>>())
        .expect("presentable")
        .disclosed_payload()
        .expect("processes");
    // Element 1 hid only its `n`, so its `a` is in the clear and comes with it.
    assert_eq!(shown["people"], json!([{ "n": "B", "a": 2 }]));
}
