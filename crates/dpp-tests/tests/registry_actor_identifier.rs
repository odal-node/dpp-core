//! The registry's operator and facility identifiers, read as EN 18219 clause 6
//! identifiers.
//!
//! `OperatorIdentifier` and `FacilityIdentifier` say their scheme in a free string.
//! `actor_identifier` reads that string through `ActorIdentifier::from_legacy`, the
//! one place those strings are read, and these tests hold the registry to it from
//! the outside: what it places, what it says it cannot, and that it does not
//! quietly change what `validate` accepts.

use dpp_domain::{ActorIdentifier, ActorIdentifierError, GlnAi, Icd};
use dpp_registry::{FacilityIdentifier, OperatorIdentifier};

const LEI: &str = "529900T8BM49AURSDO55";
const GLN: &str = "9506000134352";

fn operator(scheme: &str, value: &str) -> OperatorIdentifier {
    OperatorIdentifier {
        scheme: scheme.into(),
        value: value.into(),
        name: "Example GmbH".into(),
        country: "DE".into(),
        did: None,
    }
}

fn facility(scheme: &str, value: &str) -> FacilityIdentifier {
    FacilityIdentifier {
        scheme: scheme.into(),
        value: value.into(),
        name: Some("Plant 3".into()),
        country: "DE".into(),
        address: None,
    }
}

#[test]
fn an_operator_lei_or_duns_is_scheme_6_and_a_gln_is_a_party() {
    assert_eq!(
        operator("lei", LEI).actor_identifier(),
        ActorIdentifier::iso6523(Icd::lei(), LEI)
    );
    assert_eq!(
        operator("duns", "123456789").actor_identifier(),
        ActorIdentifier::iso6523(Icd::duns(), "123456789")
    );
    let party = operator("gln", GLN)
        .actor_identifier()
        .expect("a party GLN");
    assert!(matches!(
        party,
        ActorIdentifier::Gs1 {
            ai: GlnAi::Party,
            ..
        }
    ));
    assert_eq!(party.en_18219_scheme(), Some(7));
}

/// The same GLN is a different identifier at a facility, and the AI is what says so.
#[test]
fn a_facility_gln_is_a_physical_location() {
    let place = facility("gln", GLN)
        .actor_identifier()
        .expect("a location GLN");
    assert!(matches!(
        place,
        ActorIdentifier::Gs1 {
            ai: GlnAi::Location,
            ..
        }
    ));
    assert_ne!(
        Some(place),
        operator("gln", GLN).actor_identifier().ok(),
        "a party GLN and a location GLN must not be the same identifier"
    );
}

/// 🚨 Unmapped is not invalid.
///
/// An EORI number, a VAT number and a DID have no clause 6 route this build can
/// establish, and `validate` has always accepted them. Reading them as unmapped
/// must not turn into refusing them: no rule yet says which identifiers must have
/// a route, and that is decided separately.
#[test]
fn an_unmapped_scheme_still_validates() {
    for (scheme, value) in [
        ("eori", "DE123456789012345"),
        ("vat", "DE123456789"),
        ("did", "did:web:example.com"),
    ] {
        let operator = operator(scheme, value);
        assert!(operator.validate().is_ok(), "{scheme} no longer validates");
        assert_eq!(
            operator.actor_identifier(),
            Err(ActorIdentifierError::Unmapped {
                scheme: scheme.to_owned()
            })
        );
    }

    let national = facility("national", "DE-BY-0042");
    assert!(national.validate().is_ok());
    assert_eq!(
        national.actor_identifier(),
        Err(ActorIdentifierError::Unmapped {
            scheme: "national".into()
        })
    );
}

/// The LEI and D-U-N-S checks have one home, so the registry and the typed
/// identifier cannot disagree about what a value is.
#[test]
fn the_registry_and_the_typed_identifier_agree_about_an_lei_and_a_duns_number() {
    for value in [
        LEI,
        "529900T8BM49AURSDO56",
        "529900t8bm49aursdo55",
        "529900T8BM49AURSDO5",
        "",
        "not-an-lei",
    ] {
        let operator = operator("lei", value);
        assert_eq!(
            operator.validate().is_ok(),
            operator.actor_identifier().is_ok(),
            "{value:?}: the registry and the typed identifier disagree about an LEI"
        );
    }
    for value in ["123456789", "12345678", "1234567890", "12345678a", ""] {
        let operator = operator("duns", value);
        assert_eq!(
            operator.validate().is_ok(),
            operator.actor_identifier().is_ok(),
            "{value:?}: the registry and the typed identifier disagree about a D-U-N-S number"
        );
    }
}

/// The operator's DID is for authentication. It is not the identifier, and having
/// one must not make an unmapped identifier look mapped.
#[test]
fn the_operators_did_is_not_the_identifier() {
    let mut operator = operator("eori", "DE123456789012345");
    operator.did = Some("did:web:example.com".into());
    assert_eq!(
        operator.actor_identifier(),
        Err(ActorIdentifierError::Unmapped {
            scheme: "eori".into()
        })
    );
}
