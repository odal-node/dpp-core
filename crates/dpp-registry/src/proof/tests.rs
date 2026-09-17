//! What the Art. 9 proof admits, refuses, and how its ninety days are counted.

use chrono::{Duration, TimeZone, Utc};

use super::{AVAILABILITY_DAYS, ProofOfRegistration};
use crate::error::RegistryValidationError;
use crate::identifiers::OperatorIdentifier;

fn operator() -> OperatorIdentifier {
    OperatorIdentifier {
        scheme: "lei".into(),
        value: "529900T8BM49AURSDO55".into(),
        name: "Example GmbH".into(),
        country: "DE".into(),
        did: None,
    }
}

fn proof() -> ProofOfRegistration {
    ProofOfRegistration {
        product_identifier: "https://id.example.com/01/09506000134352".into(),
        commodity_code: Some("850760".into()),
        operator: operator(),
        registered_at: Utc.with_ymd_and_hms(2026, 8, 1, 9, 0, 0).unwrap(),
        commission_time_stamp: "MIIC…opaque-rfc3161…".into(),
        passport_version_hash: "e3b0c44298fc1c149afbf4c8996fb924".into(),
        generated_at: Utc.with_ymd_and_hms(2026, 8, 1, 9, 0, 5).unwrap(),
        qualified_seal: "MIIF…opaque-qseal…".into(),
    }
}

#[test]
fn a_complete_proof_validates() {
    assert!(proof().validate().is_ok(), "{:?}", proof().validate());
}

/// Art. 9(2) lists five points and (b) is the only one qualified by *"where
/// relevant"*. The rest cannot be empty, because a proof missing any of them
/// evidences less than the article says a proof evidences.
#[test]
fn each_mandatory_point_is_required() {
    /// Empties one of the points, so each case names the field it breaks.
    type Break = fn(&mut ProofOfRegistration);

    let cases: [(&str, Break); 4] = [
        ("productIdentifier", |p| {
            p.product_identifier = String::new();
        }),
        ("commissionTimeStamp", |p| {
            // Whitespace, not empty: a point carrying only spaces is as absent
            // as one carrying nothing, and `is_empty` alone would accept it.
            p.commission_time_stamp = "   ".into();
        }),
        ("passportVersionHash", |p| {
            p.passport_version_hash = String::new();
        }),
        ("qualifiedSeal", |p| p.qualified_seal = String::new()),
    ];

    for (field, break_it) in cases {
        let mut p = proof();
        break_it(&mut p);
        assert!(
            matches!(
                p.validate(),
                Err(RegistryValidationError::MissingRequiredField(ref f)) if f == field
            ),
            "an empty {field} was accepted"
        );
    }
}

/// Point (b) is *"where relevant"*, so absent is lawful — and malformed is not.
#[test]
fn the_commodity_code_is_optional_and_checked_when_present() {
    let mut absent = proof();
    absent.commodity_code = None;
    assert!(absent.validate().is_ok());

    let mut malformed = proof();
    malformed.commodity_code = Some("85".into());
    assert!(matches!(
        malformed.validate(),
        Err(RegistryValidationError::InvalidCommodityCode { .. })
    ));
}

/// 🚨 A registration cannot postdate the generation of the proof that reports it.
///
/// Art. 9(2)(d) is the registration of the *latest version at generation time*,
/// so the two are ordered by the article itself. It is the one internal
/// contradiction the five points can express between them, and a proof carrying
/// it describes a sequence of events that did not happen.
#[test]
fn a_registration_after_its_own_proof_is_refused() {
    let mut p = proof();
    p.registered_at = p.generated_at + Duration::seconds(1);
    assert!(
        p.validate().is_err(),
        "a proof generated before the registration it reports was accepted"
    );

    // Equal is fine: nothing in the article requires a gap, and a registry that
    // generates a proof in the same instant has done nothing wrong.
    p.registered_at = p.generated_at;
    assert!(p.validate().is_ok());
}

/// Art. 9(4): ninety calendar days from generation.
#[test]
fn availability_runs_ninety_days_from_generation() {
    let p = proof();
    assert_eq!(AVAILABILITY_DAYS, 90);
    assert_eq!(
        p.available_until(),
        p.generated_at + Duration::days(90),
        "the window runs from generation, not from registration"
    );

    // 🚨 From *generation*, and the two differ. Pinned because reading the
    // window off `registered_at` is the plausible mistake, and for this proof it
    // would be wrong by five seconds — small enough to survive a spot check and
    // arbitrarily large for a proof generated long after its registration.
    assert_ne!(p.available_until(), p.registered_at + Duration::days(90));
}

/// The boundary, and the reason the method is named for availability.
#[test]
fn availability_lapses_without_the_proof_ceasing_to_be_evidence() {
    let p = proof();

    assert!(p.is_available_at(p.generated_at));
    assert!(p.is_available_at(p.available_until() - Duration::seconds(1)));
    // Exactly ninety days later the registry is no longer obliged to serve it.
    assert!(!p.is_available_at(p.available_until()));
    assert!(!p.is_available_at(p.available_until() + Duration::days(1)));

    // …and the document is unchanged by that. Nothing in the type expires, the
    // seal is still carried, and `validate` still passes — which is the whole
    // distinction Art. 9(4) draws between what the Commission makes available
    // and what the proof is.
    assert!(p.validate().is_ok());
}

/// The five points are a floor. Art. 9(2) says *"at least"*, so a document
/// carrying more than the article compels is lawful and must still read.
#[test]
fn unknown_fields_do_not_refuse_a_lawful_proof() {
    let json = serde_json::json!({
        "productIdentifier": "https://id.example.com/01/09506000134352",
        "operator": {
            "scheme": "lei",
            "value": "529900T8BM49AURSDO55",
            "name": "Example GmbH",
            "country": "DE",
        },
        "registeredAt": "2026-08-01T09:00:00Z",
        "commissionTimeStamp": "MIIC…opaque…",
        "passportVersionHash": "e3b0c44298fc1c149afbf4c8996fb924",
        "generatedAt": "2026-08-01T09:00:05Z",
        "qualifiedSeal": "MIIF…opaque…",
        // Not in Art. 9(2). The Commission may send it; refusing the document
        // over it would reject a proof the article expressly permits.
        "registrationIdentifier": "EU-DPP-0001",
    });

    let parsed: ProofOfRegistration =
        serde_json::from_value(json).expect("a proof carrying more than the minimum must read");
    assert!(parsed.validate().is_ok());
    assert_eq!(parsed.commodity_code, None);
}
