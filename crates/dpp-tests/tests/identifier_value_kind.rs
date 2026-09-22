//! Tripwire: the three surfaces that label a product identifier agree on what
//! to call each kind of value.
//!
//! `ProductIdentifier::value_kind` is the one home, and two published surfaces
//! read from it in spirit while stating the strings themselves:
//!
//! - `dpp_registry::{SCHEME_GTIN, SCHEME_IDENTIFICATION_LINK, SCHEME_DID}` —
//!   the EU registry wire `scheme`;
//! - the AAS `specificAssetId` name, derived from `value_kind` directly.
//!
//! 🚨 **These are not the clause 5 scheme tags.** The domain enum's serde tag
//! for scheme 1 is `"gs1"`, which names the scheme that issued the identifier;
//! `value_kind` names *what the value is*, which is `"gtin"`. Two vocabularies,
//! deliberately not collapsed — they coincide for schemes 2 and 3 and differ
//! for scheme 1, which is exactly where conflating them would go wrong without
//! failing.
//!
//! The registry constants stay written out, because they are published API and
//! documented as `RegistryBasis::Assumed` — a claim about the registry, not
//! about us. This asserts they have not drifted from the value they label.

use dpp_domain::Gtin;
use dpp_domain::identifier::ProductIdentifier;
use dpp_registry::{SCHEME_DID, SCHEME_GTIN, SCHEME_IDENTIFICATION_LINK};

fn one_of_each() -> Vec<ProductIdentifier> {
    vec![
        ProductIdentifier::gs1(Gtin::parse("09506000134352").expect("a valid GTIN")),
        ProductIdentifier::identification_link("https://id.acme.example.com/b/1")
            .expect("a valid link"),
        ProductIdentifier::did("did:web:acme.example.com:b:1").expect("a valid DID"),
    ]
}

#[test]
fn value_kind_matches_the_registry_scheme_it_labels() {
    let expected = [SCHEME_GTIN, SCHEME_IDENTIFICATION_LINK, SCHEME_DID];
    for (identifier, scheme) in one_of_each().iter().zip(expected) {
        assert_eq!(
            identifier.value_kind(),
            scheme,
            "value_kind and the registry scheme disagree for {identifier:?}"
        );
    }
}

#[test]
fn value_kind_is_not_the_clause_5_scheme_tag() {
    // The distinction, asserted rather than left to a comment. If scheme 1 ever
    // answers "gs1" here, every surface labelling a value has silently switched
    // vocabulary.
    let gs1 = &one_of_each()[0];
    assert_eq!(gs1.value_kind(), "gtin");
    let tag = serde_json::to_value(gs1).expect("serialises");
    assert_eq!(
        tag["scheme"], "gs1",
        "the persisted tag names the scheme, not the value kind"
    );
}

#[test]
fn only_the_two_self_issuing_schemes_are_already_uris() {
    // What decides whether `globalAssetId` wraps the value. A GTIN is a bare
    // number; wrapping the other two produces a URN whose namespace-specific
    // string is another URI scheme.
    let ids = one_of_each();
    assert!(!ids[0].is_uri(), "a bare GTIN is not a URI");
    assert!(ids[1].is_uri(), "an identification link is a URL");
    assert!(ids[2].is_uri(), "a DID is a URI");
}

/// 🚨 The reserved list and the derived names are two statements of one set.
///
/// `AssetIdentity::Named` refuses the names a scheme derives, and that refusal
/// is only as complete as the list. Add a clause 5 scheme — the enum is
/// `#[non_exhaustive]` precisely because that is expected — and `value_kind`
/// would return a fourth string that the escape hatch could then claim, which
/// is the defect the reservation exists to prevent, reopened by omission.
#[test]
fn every_derived_name_is_reserved_against_the_escape_hatch() {
    for identifier in one_of_each() {
        assert!(
            dpp_aas::RESERVED_ASSET_ID_NAMES.contains(&identifier.value_kind()),
            "{} is a derived asset id name and must be reserved",
            identifier.value_kind()
        );
    }
    // And nothing is reserved that no scheme derives, which would refuse a
    // caller-chosen name for no reason.
    let derived: Vec<&str> = one_of_each().iter().map(|i| i.value_kind()).collect();
    for reserved in dpp_aas::RESERVED_ASSET_ID_NAMES {
        assert!(
            derived.contains(&reserved),
            "{reserved} is reserved but no scheme derives it"
        );
    }
}
