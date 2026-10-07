//! Wire shape of the personal-data statement types.

use serde_json::json;

use super::{HeldOutside, LawfulBasis, PersonalDataRecordId, PersonalDataStatement};

#[test]
fn nothing_held_is_one_tag() {
    let wire = serde_json::to_value(PersonalDataStatement::NothingHeld).unwrap();
    assert_eq!(wire, json!({ "held": "nothing" }));
    let back: PersonalDataStatement = serde_json::from_value(wire).unwrap();
    assert_eq!(back, PersonalDataStatement::NothingHeld);
}

#[test]
fn held_outside_carries_its_basis_and_record_beside_the_tag() {
    let statement = PersonalDataStatement::HeldOutside(HeldOutside::new(
        LawfulBasis::Consent,
        PersonalDataRecordId::new("rec-1"),
    ));
    let wire = serde_json::to_value(&statement).unwrap();
    assert_eq!(
        wire,
        json!({ "held": "outside", "lawfulBasis": "consent", "record": "rec-1" })
    );
    let back: PersonalDataStatement = serde_json::from_value(wire).unwrap();
    assert_eq!(back, statement);
}

/// The six points of GDPR Art. 6(1), each under its own name. A wire name is
/// what an operator writes into a signed passport, so it must never move.
#[test]
fn every_lawful_basis_has_a_stable_wire_name() {
    for (basis, wire) in [
        (LawfulBasis::Consent, "consent"),
        (LawfulBasis::Contract, "contract"),
        (LawfulBasis::LegalObligation, "legalObligation"),
        (LawfulBasis::VitalInterests, "vitalInterests"),
        (LawfulBasis::PublicTask, "publicTask"),
        (LawfulBasis::LegitimateInterests, "legitimateInterests"),
    ] {
        assert_eq!(serde_json::to_value(basis).unwrap(), json!(wire));
    }
}

#[test]
fn an_unknown_form_is_refused() {
    let err = serde_json::from_value::<PersonalDataStatement>(json!({ "held": "inside" }));
    assert!(err.is_err(), "there is no form that keeps the data inside");
}

#[test]
fn a_record_id_is_its_string_on_the_wire() {
    let id = PersonalDataRecordId::new("rec-7");
    assert_eq!(serde_json::to_value(&id).unwrap(), json!("rec-7"));
    assert_eq!(id.to_string(), "rec-7");
}
