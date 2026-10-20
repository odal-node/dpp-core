//! What an operator or facility identifier is read as, and what it is refused for.

use serde_json::json;

use super::{ActorIdentifier, ActorIdentifierError, ActorPosition, GlnAi, Icd};
use crate::identifier::Gln;

/// An LEI whose ISO 7064 check digits are correct.
const LEI: &str = "529900T8BM49AURSDO55";

/// A GLN whose GS1 check digit is correct.
const GLN: &str = "9506000134352";

fn gln() -> Gln {
    Gln::parse(GLN).expect("a valid GLN literal")
}

#[test]
fn an_icd_is_four_ascii_digits_and_nothing_else() {
    for good in ["0199", "0060", "0088", "9999", "0000"] {
        assert_eq!(Icd::parse(good).expect(good).as_str(), good);
    }
    for bad in ["", "199", "01999", "019a", " 199", "０１９９", "0199 "] {
        assert_eq!(
            Icd::parse(bad),
            Err(ActorIdentifierError::InvalidIcd(bad.to_owned())),
            "{bad:?} was accepted as an ICD"
        );
    }
    assert_eq!(Icd::lei().as_str(), "0199");
    assert_eq!(Icd::duns().as_str(), "0060");
}

/// 🚨 The shape check is on the ICD, because that is what names the scheme whose
/// syntax this crate holds.
#[test]
fn an_lei_and_a_duns_number_are_checked_under_their_own_icd() {
    assert!(ActorIdentifier::iso6523(Icd::lei(), LEI).is_ok());
    assert!(ActorIdentifier::iso6523(Icd::duns(), "123456789").is_ok());

    // A lower-case LEI is refused, not folded: what is stored is what is printed.
    for bad in [
        "529900t8bm49aursdo55",
        "529900T8BM49AURSDO56",
        "529900T8BM49AURSDO5",
        "",
    ] {
        let refused = ActorIdentifier::iso6523(Icd::lei(), bad).expect_err(bad);
        assert!(
            matches!(
                refused,
                ActorIdentifierError::InvalidValue { .. } | ActorIdentifierError::BlankValue
            ),
            "{bad:?}: {refused:?}"
        );
    }
    assert_eq!(
        ActorIdentifier::iso6523(Icd::duns(), "12345678"),
        Err(ActorIdentifierError::InvalidValue {
            icd: "0060".into(),
            value: "12345678".into(),
        })
    );
    assert_eq!(
        ActorIdentifier::iso6523(Icd::lei(), "   "),
        Err(ActorIdentifierError::BlankValue)
    );
}

/// The syntax of any other scheme is the scheme's own, and this crate does not
/// hold it, so it says nothing about a value beyond that there is one.
#[test]
fn any_other_icd_takes_a_non_blank_value_and_no_further_check() {
    let norway = Icd::parse("0192").expect("an ICD");
    assert!(ActorIdentifier::iso6523(norway.clone(), "987654321").is_ok());
    assert!(ActorIdentifier::iso6523(norway.clone(), "anything at all").is_ok());
    assert_eq!(
        ActorIdentifier::iso6523(norway, " "),
        Err(ActorIdentifierError::BlankValue)
    );
}

#[test]
fn a_gln_carries_the_ai_that_says_what_it_names() {
    let party = ActorIdentifier::gs1(GlnAi::Party, gln());
    let place = ActorIdentifier::gs1(GlnAi::Location, gln());

    // The same thirteen digits, told apart by the AI and not by where they were found.
    assert_ne!(party, place);
    assert_eq!(party.value(), GLN);
    assert_eq!(place.value(), GLN);
    assert_eq!(GlnAi::Party.digits(), "417");
    assert_eq!(GlnAi::Location.digits(), "414");
}

#[test]
fn each_identifier_says_which_clause_6_scheme_it_follows() {
    assert_eq!(
        ActorIdentifier::iso6523(Icd::lei(), LEI)
            .expect("an LEI")
            .en_18219_scheme(),
        6
    );
    assert_eq!(
        ActorIdentifier::gs1(GlnAi::Party, gln()).en_18219_scheme(),
        7
    );
}

/// 🚨 The wire form is a persisted shape in waiting, so it is written out and
/// held. A rename here would be a rename of what a signed passport says.
#[test]
fn the_wire_form_is_exactly_this() {
    let scheme_6 = ActorIdentifier::iso6523(Icd::lei(), LEI).expect("an LEI");
    let scheme_7 = ActorIdentifier::gs1(GlnAi::Party, gln());

    assert_eq!(
        serde_json::to_value(&scheme_6).expect("serialises"),
        json!({ "scheme": "icd", "icd": "0199", "value": LEI })
    );
    assert_eq!(
        serde_json::to_value(&scheme_7).expect("serialises"),
        json!({ "scheme": "gs1", "ai": "417", "gln": GLN })
    );
    assert_eq!(
        serde_json::to_value(ActorIdentifier::gs1(GlnAi::Location, gln())).expect("serialises"),
        json!({ "scheme": "gs1", "ai": "414", "gln": GLN })
    );

    for identifier in [scheme_6, scheme_7] {
        let wire = serde_json::to_value(&identifier).expect("serialises");
        let read: ActorIdentifier = serde_json::from_value(wire).expect("reads back");
        assert_eq!(read, identifier);
    }
}

/// 🚨 Reading is not issuing. A document is held to its shape, and its value is
/// checked by [`ActorIdentifier::validate`] when someone asks.
///
/// An operator identifier is in a passport that can arrive from another operator,
/// signed and unrewritable. A reader that refused a value it could not have refused
/// when it was written would refuse correct data permanently.
#[test]
fn a_document_is_read_as_it_is_and_its_value_is_checked_when_asked() {
    let wire = json!({ "scheme": "icd", "icd": "0199", "value": "not-an-lei" });
    let read: ActorIdentifier = serde_json::from_value(wire).expect("the shape is right");

    assert_eq!(
        read.value(),
        "not-an-lei",
        "reading must not alter the value"
    );
    assert_eq!(
        read.validate(),
        Err(ActorIdentifierError::InvalidValue {
            icd: "0199".into(),
            value: "not-an-lei".into(),
        })
    );

    // The shape still binds: an ICD is four digits, and a GLN is a GLN.
    for wire in [
        json!({ "scheme": "icd", "icd": "199", "value": "x" }),
        json!({ "scheme": "icd", "icd": "0199" }),
        json!({ "scheme": "gs1", "ai": "417", "gln": "9506000134353" }),
        json!({ "scheme": "gs1", "ai": "416", "gln": GLN }),
        json!({ "scheme": "doi", "value": "10.1000/1" }),
    ] {
        assert!(
            serde_json::from_value::<ActorIdentifier>(wire.clone()).is_err(),
            "{wire} was read"
        );
    }
}

/// The table in [`ActorIdentifier::from_legacy`], row by row.
#[test]
fn a_legacy_lei_duns_or_gln_is_placed() {
    assert_eq!(
        ActorIdentifier::from_legacy("lei", LEI, ActorPosition::Operator),
        ActorIdentifier::iso6523(Icd::lei(), LEI)
    );
    assert_eq!(
        ActorIdentifier::from_legacy("duns", "123456789", ActorPosition::Operator),
        ActorIdentifier::iso6523(Icd::duns(), "123456789")
    );
    assert_eq!(
        ActorIdentifier::from_legacy("gln", GLN, ActorPosition::Operator),
        Ok(ActorIdentifier::gs1(GlnAi::Party, gln()))
    );
}

/// The one thing a legacy `gln` string does not say, so the caller does.
#[test]
fn a_legacy_gln_is_a_party_at_an_operator_and_a_location_at_a_facility() {
    assert_eq!(
        ActorIdentifier::from_legacy("gln", GLN, ActorPosition::Operator),
        Ok(ActorIdentifier::gs1(GlnAi::Party, gln()))
    );
    assert_eq!(
        ActorIdentifier::from_legacy("gln", GLN, ActorPosition::Facility),
        Ok(ActorIdentifier::gs1(GlnAi::Location, gln()))
    );
    // An LEI and a D-U-N-S number name an organisation, wherever they are found.
    for position in [ActorPosition::Operator, ActorPosition::Facility] {
        assert_eq!(
            ActorIdentifier::from_legacy("lei", LEI, position),
            ActorIdentifier::iso6523(Icd::lei(), LEI)
        );
    }
}

/// 🚨 What this build cannot place is said to be unplaced, and is never guessed.
///
/// An EU-wide VAT number has no ICD, none was found for an EORI number, and a DID
/// is scheme 8 only with a register-issued credential. Reading any of them as
/// something would be a guess presented as a mapping.
#[test]
fn a_scheme_with_no_known_clause_6_route_is_unmapped_and_not_guessed() {
    for scheme in [
        "vat",
        "eori",
        "did",
        "national",
        "passport_id",
        "",
        "iso6523",
        "gs1",
    ] {
        for position in [ActorPosition::Operator, ActorPosition::Facility] {
            assert_eq!(
                ActorIdentifier::from_legacy(scheme, "DE123456789", position),
                Err(ActorIdentifierError::Unmapped {
                    scheme: scheme.to_owned()
                }),
                "{scheme:?} was placed"
            );
        }
    }
}

/// The registry matches its scheme strings exactly, and so does this.
#[test]
fn a_legacy_scheme_string_is_matched_exactly() {
    for scheme in ["LEI", "Lei", " lei", "lei ", "GLN", "DUNS"] {
        assert_eq!(
            ActorIdentifier::from_legacy(scheme, LEI, ActorPosition::Operator),
            Err(ActorIdentifierError::Unmapped {
                scheme: scheme.to_owned()
            }),
            "{scheme:?} was placed"
        );
    }
}

/// A scheme this build can place is placed, and then the value has to fit it.
#[test]
fn a_placed_scheme_with_a_value_that_does_not_fit_is_refused_for_the_value() {
    assert!(matches!(
        ActorIdentifier::from_legacy("lei", "529900T8BM49AURSDO56", ActorPosition::Operator),
        Err(ActorIdentifierError::InvalidValue { .. })
    ));
    assert_eq!(
        ActorIdentifier::from_legacy("duns", "", ActorPosition::Operator),
        Err(ActorIdentifierError::BlankValue)
    );
    assert!(matches!(
        ActorIdentifier::from_legacy("gln", "9506000134353", ActorPosition::Facility),
        Err(ActorIdentifierError::InvalidGln { .. })
    ));
    assert!(matches!(
        ActorIdentifier::from_legacy("gln", "not a gln", ActorPosition::Operator),
        Err(ActorIdentifierError::InvalidGln { .. })
    ));
}
