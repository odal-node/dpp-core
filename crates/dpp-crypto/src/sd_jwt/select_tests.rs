//! Choosing what to present by claim path (`select.rs`): what a path reveals, and the
//! ways a request fails.
//!
//! Tokens built by hand, one behaviour each. Where an array element is lives in
//! `select_position_tests.rs`, and the RFC's own choices, the path's JSON form and a
//! property test against a model are in `select_model_tests.rs`.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::tests::stub_jwt;
use super::{
    ClaimPath, Disclosure, PathSegment, SdJwt, SdJwtError, build_payload, conceal, conceal_elements,
};

pub(super) fn object(value: Value) -> Map<String, Value> {
    match value {
        Value::Object(map) => map,
        other => panic!("not an object: {other}"),
    }
}

pub(super) fn digests_of(disclosures: &[Disclosure]) -> BTreeSet<String> {
    disclosures.iter().map(Disclosure::digest).collect()
}

pub(super) fn chosen(token: &SdJwt, paths: &[ClaimPath]) -> BTreeSet<String> {
    token
        .digests_for_paths(paths)
        .expect("the paths are in the token")
        .into_iter()
        .collect()
}

/// The digest of the Disclosure that hides `name` in `disclosures`.
pub(super) fn digest_named(disclosures: &[Disclosure], name: &str) -> String {
    let found: Vec<_> = disclosures
        .iter()
        .filter(|d| d.claim_name() == Some(name))
        .collect();
    assert_eq!(found.len(), 1, "expected one Disclosure for {name}");
    found[0].digest()
}

/// What the tests below choose from:
///
/// ```text
/// iss          in the clear
/// given_name   hidden
/// address      hidden, and its `region` hidden inside it
///                { street, region*, country }
/// list         in the clear: [ a*, b, c*, d* ]
/// ```
///
/// Returns the token and, for each Disclosure the tests name, its digest.
pub(super) struct Fixture {
    pub(super) token: SdJwt,
    pub(super) given_name: String,
    pub(super) address: String,
    pub(super) region: String,
    /// The Disclosures of `list`'s elements `a`, `c` and `d`.
    pub(super) a: String,
    pub(super) c: String,
    pub(super) d: String,
}

pub(super) fn fixture() -> Fixture {
    let (address_kept, region_d) = conceal(
        &object(json!({ "street": "1 Main St", "region": "Anystate", "country": "US" })),
        |name| name == "region",
    )
    .expect("conceals");
    let (list, element_d) = conceal_elements(
        &[json!("a"), json!("b"), json!("c"), json!("d")],
        |position, _| position != 1,
    );
    let (root, root_d) = conceal(
        &object(json!({
            "iss": "https://issuer.example",
            "given_name": "Erika",
            "address": Value::Object(address_kept),
            "list": list,
        })),
        |name| name == "given_name" || name == "address",
    )
    .expect("conceals");

    let digest_of_element = |value: &str| {
        element_d
            .iter()
            .find(|d| d.claim_value() == &json!(value))
            .expect("hidden")
            .digest()
    };
    let (a, c, d) = (
        digest_of_element("a"),
        digest_of_element("c"),
        digest_of_element("d"),
    );
    let mut disclosures = root_d;
    disclosures.extend(region_d);
    disclosures.extend(element_d);
    Fixture {
        given_name: digest_named(&disclosures, "given_name"),
        address: digest_named(&disclosures, "address"),
        region: digest_named(&disclosures, "region"),
        a,
        c,
        d,
        token: SdJwt::new(stub_jwt(&build_payload(root, true)), disclosures),
    }
}

pub(super) fn set<const N: usize>(digests: [&String; N]) -> BTreeSet<String> {
    digests.into_iter().cloned().collect()
}

// --- what a path reveals ------------------------------------------------------

#[test]
fn a_top_level_claim_is_chosen_by_its_name() {
    let f = fixture();
    assert_eq!(
        chosen(&f.token, &[ClaimPath::claim("given_name")]),
        set([&f.given_name])
    );
}

#[test]
fn a_claim_in_the_clear_needs_no_disclosure_and_is_not_an_error() {
    let f = fixture();
    assert!(
        f.token
            .digests_for_paths(&[ClaimPath::claim("iss")])
            .expect("in the token")
            .is_empty()
    );
    // `b` is the one element of `list` that was left in the clear.
    assert!(
        f.token
            .digests_for_paths(&[ClaimPath::claim("list").index(1)])
            .expect("in the token")
            .is_empty()
    );
}

#[test]
fn a_nested_claim_brings_the_claims_above_it_and_no_others() {
    let f = fixture();
    let digests = f
        .token
        .digests_for_paths(&[ClaimPath::claim("address").key("region")])
        .expect("in the token");
    assert_eq!(
        digests.iter().cloned().collect::<BTreeSet<_>>(),
        set([&f.address, &f.region])
    );

    let shown = f
        .token
        .present(&digests.iter().map(String::as_str).collect::<Vec<_>>())
        .expect("the parents are there, so it is presentable")
        .disclosed_payload()
        .expect("processes");
    assert_eq!(shown["address"]["region"], json!("Anystate"));
    assert!(!shown.contains_key("given_name"));
    assert_eq!(shown["list"], json!(["b"]), "no element of list was chosen");
}

#[test]
fn a_container_is_chosen_alone_and_what_is_hidden_inside_it_stays_hidden() {
    let f = fixture();
    let digests = f
        .token
        .digests_for_paths(&[ClaimPath::claim("address")])
        .expect("in the token");
    assert_eq!(digests, vec![f.address.clone()]);

    let shown = f
        .token
        .present(&[f.address.as_str()])
        .expect("presentable")
        .disclosed_payload()
        .expect("processes");
    // What was in the clear inside the claim comes with it, and its hidden member
    // does not.
    assert_eq!(shown["address"]["street"], json!("1 Main St"));
    assert!(shown["address"].get("region").is_none());
}

// --- several paths, and the ways a request fails ------------------------------

#[test]
fn several_paths_are_one_selection_each_disclosure_once_in_the_tokens_order() {
    let f = fixture();
    let digests = f
        .token
        .digests_for_paths(&[
            ClaimPath::claim("list").index(2),
            ClaimPath::claim("address").key("region"),
            ClaimPath::claim("list").index(2),
            ClaimPath::claim("address"),
        ])
        .expect("in the token");

    let in_token_order: Vec<String> = f
        .token
        .disclosures()
        .iter()
        .map(Disclosure::digest)
        .filter(|digest| [f.c.clone(), f.address.clone(), f.region.clone()].contains(digest))
        .collect();
    assert_eq!(digests, in_token_order);
}

#[test]
fn a_path_the_token_does_not_hold_is_refused_and_named() {
    let f = fixture();
    for (path, shown) in [
        (ClaimPath::claim("nope"), r#"["nope"]"#),
        (ClaimPath::claim("list").index(9), r#"["list",9]"#),
        (
            ClaimPath::claim("address").key("nope"),
            r#"["address","nope"]"#,
        ),
        // A key into a scalar, a key into an array, an index into an object.
        (
            ClaimPath::claim("given_name").key("x"),
            r#"["given_name","x"]"#,
        ),
        (ClaimPath::claim("list").key("a"), r#"["list","a"]"#),
        (ClaimPath::claim("address").index(0), r#"["address",0]"#),
    ] {
        assert_eq!(
            f.token.digests_for_paths(&[path]).unwrap_err(),
            SdJwtError::ClaimNotInToken(shown.to_owned())
        );
    }
}

#[test]
fn one_path_that_is_not_there_fails_the_whole_request() {
    let f = fixture();
    assert_eq!(
        f.token
            .digests_for_paths(&[ClaimPath::claim("given_name"), ClaimPath::claim("nope")])
            .unwrap_err(),
        SdJwtError::ClaimNotInToken(r#"["nope"]"#.to_owned()),
        "a Holder asked for two claims and gets neither, not one of them in silence"
    );
}

#[test]
fn a_wildcard_that_fits_some_elements_is_not_an_error() {
    let (first, first_d) = conceal(&object(json!({ "type": "BSc" })), |_| true).unwrap();
    let second = object(json!({ "year": 2020 }));
    let token = SdJwt::new(
        stub_jwt(&build_payload(
            object(json!({ "degrees": [Value::Object(first), Value::Object(second)] })),
            true,
        )),
        first_d,
    );
    let degrees = ClaimPath::new(vec![
        PathSegment::Key("degrees".to_owned()),
        PathSegment::AllElements,
        PathSegment::Key("type".to_owned()),
    ])
    .unwrap();
    assert_eq!(token.digests_for_paths(&[degrees]).unwrap().len(), 1);
}

#[test]
fn the_names_the_mechanism_owns_are_not_claims() {
    let f = fixture();
    for path in [
        ClaimPath::claim("_sd"),
        ClaimPath::claim("_sd_alg"),
        ClaimPath::claim("address").key("_sd"),
        ClaimPath::claim("list").key("..."),
    ] {
        assert!(
            matches!(
                f.token.digests_for_paths(&[path]).unwrap_err(),
                SdJwtError::ClaimNotInToken(_)
            ),
            "the mechanism's own names reached a claim"
        );
    }
}

#[test]
fn a_token_that_carries_a_key_binding_jwt_cannot_be_chosen_from() {
    let f = fixture();
    let bound = SdJwt::parse(&format!("{}e30.e30.e30", f.token.serialise())).expect("parses");
    assert_eq!(
        bound
            .digests_for_paths(&[ClaimPath::claim("given_name")])
            .unwrap_err(),
        SdJwtError::KeyBindingAttached
    );
}

#[test]
fn a_token_that_cannot_be_read_is_not_chosen_from() {
    let f = fixture();
    let forged = Disclosure::new("forged", json!(1)).unwrap();
    let mut disclosures = f.token.disclosures().to_vec();
    disclosures.push(forged);
    let tampered = SdJwt::new(f.token.jwt().to_owned(), disclosures);
    assert_eq!(
        tampered
            .digests_for_paths(&[ClaimPath::claim("given_name")])
            .unwrap_err(),
        SdJwtError::UnusedDisclosures(1)
    );
}
