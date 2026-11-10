//! End-of-life events: what they cite, and what they still read.

use serde_json::json;

use super::*;
use crate::passport::PassportId;
use crate::unsold_goods::DiscardReason;

/// A destruction citing `derogation`, declared now.
fn destroyed(derogation: Option<Derogation>) -> EolEvent {
    EolEvent::new(
        PassportId::new(),
        DeactivationReason::Destroyed { derogation },
        "did:web:r",
    )
}

/// Every end of life but a destruction owes no derogation, whatever the ban says.
#[test]
fn recycled_roundtrips_and_is_never_missing_a_derogation() {
    let e = EolEvent::new(PassportId::new(), DeactivationReason::Recycled, "did:web:r");
    assert!(!e.missing_derogation(true));
    let v = serde_json::to_value(&e).unwrap();
    assert_eq!(v["reason"]["kind"], "recycled");
    let back: EolEvent = serde_json::from_value(v).unwrap();
    assert_eq!(back, e);
}

/// A point of Del. Reg. (EU) 2026/296 Art. 2 is written as the bare string, and
/// satisfies the requirement where the ban applies.
#[test]
fn a_destruction_citing_a_point_writes_it_as_a_string() {
    let e = destroyed(Some(Derogation::Point(
        DiscardReason::DamagedOrContaminated,
    )));
    let v = serde_json::to_value(&e).unwrap();

    assert_eq!(v["reason"]["kind"], "destroyed");
    assert_eq!(v["reason"]["derogation"], "damagedOrContaminated");
    assert_eq!(serde_json::from_value::<EolEvent>(v).unwrap(), e);
    assert!(!e.missing_derogation(true));
}

/// The ban reaches only some destructions, so a destruction that cites nothing is a
/// real record: it writes no key, and it is missing a derogation only where the ban
/// applies.
#[test]
fn a_destruction_citing_nothing_is_missing_a_derogation_only_where_the_ban_applies() {
    let e = destroyed(None);
    let v = serde_json::to_value(&e).unwrap();

    assert_eq!(v["reason"], json!({ "kind": "destroyed" }));
    assert_eq!(serde_json::from_value::<EolEvent>(v).unwrap(), e);
    assert!(e.missing_derogation(true));
    assert!(!e.missing_derogation(false));
}

/// Where the ban does not reach, what is cited does not matter.
#[test]
fn a_destruction_the_ban_does_not_reach_is_never_missing_one() {
    for derogation in [
        None,
        Some(Derogation::Point(DiscardReason::DangerousProduct)),
        Some(Derogation::Legacy(DerogationRef {
            category: "anything".into(),
            act_citation: None,
        })),
    ] {
        assert!(!destroyed(derogation).missing_derogation(false));
    }
}

/// An event written when the citation was free text: the frozen shape. It must read,
/// and write back as the same JSON, because a stored event is read again and a
/// fetched one is signed by someone else.
fn written_with_free_text(derogation: serde_json::Value) -> serde_json::Value {
    json!({
        "passportId": "0190f3a0-0000-7000-8000-000000000001",
        "reason": { "kind": "destroyed", "derogation": derogation },
        "declaredBy": "did:web:r",
        "declaredAt": "2026-08-01T09:30:00Z"
    })
}

/// Both free-text forms, with and without the act citation, read as `Legacy` and
/// write back as the JSON they came from.
#[test]
fn an_event_written_with_a_free_text_citation_still_reads_and_writes_back_unchanged() {
    for derogation in [
        json!({ "category": "health-and-safety", "actCitation": "Delegated Reg. (EU) 2026/xxx" }),
        json!({ "category": "safety" }),
    ] {
        let old = written_with_free_text(derogation.clone());
        let read: EolEvent = serde_json::from_value(old.clone()).expect("an old event reads");

        assert_eq!(
            serde_json::to_value(&read).unwrap(),
            old,
            "{derogation} did not write back as it was read"
        );
        let DeactivationReason::Destroyed { derogation: cited } = &read.reason else {
            panic!("read as {:?}", read.reason);
        };
        assert!(matches!(cited, Some(Derogation::Legacy(_))), "{cited:?}");
    }
}

/// Free text cannot be mapped to a point without inventing one, so it is never taken
/// for a derogation: an old destruction the ban reaches is reported.
#[test]
fn a_free_text_citation_is_not_a_derogation() {
    let old = written_with_free_text(json!({ "category": "health-and-safety" }));
    let read: EolEvent = serde_json::from_value(old).unwrap();

    assert!(read.missing_derogation(true));
    assert!(!read.missing_derogation(false));
}

/// The new form is closed: a string that is not a point of Art. 2 does not read, so a
/// typo cannot pass for a derogation.
#[test]
fn a_string_that_is_not_a_point_of_article_2_does_not_read() {
    let wrong = written_with_free_text(json!("end_of_season"));
    assert!(serde_json::from_value::<EolEvent>(wrong).is_err());
}

/// Only a cited point of Art. 2 names a point.
#[test]
fn a_legacy_citation_has_no_point_and_a_cited_point_has_one() {
    let point = Derogation::Point(DiscardReason::LicensedPeriodExpired);
    let legacy = Derogation::Legacy(DerogationRef {
        category: "licence".into(),
        act_citation: None,
    });
    assert_eq!(point.point(), Some(DiscardReason::LicensedPeriodExpired));
    assert_eq!(legacy.point(), None);
}
