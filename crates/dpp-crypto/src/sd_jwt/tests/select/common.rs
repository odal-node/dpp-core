//! Helpers the claim-path selection tests share.

use std::collections::BTreeSet;

use serde_json::{Map, Value, json};

use super::super::super::{ClaimPath, Disclosure, SdJwt, build_payload, conceal, conceal_elements};
use super::super::common::stub_jwt;

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
