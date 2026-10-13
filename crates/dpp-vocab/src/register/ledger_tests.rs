//! The term ledger: a verified vocabulary permits the terms read, not its namespace.

use super::*;
use crate::vocabulary::{Layer, ReadTerm, Status, Vocabulary};

fn register() -> VocabularyRegister {
    VocabularyRegister::new()
}

/// A record shaped like the embedded ones, for a case they do not contain.
fn record(
    key: &str,
    status: Status,
    layer: Layer,
    namespace: Option<&str>,
    terms: &[&str],
) -> Vocabulary {
    Vocabulary {
        key: key.to_owned(),
        title: key.to_owned(),
        authority: "test".to_owned(),
        layer,
        namespace_iri: namespace.map(str::to_owned),
        used_for: "test".to_owned(),
        licence: None,
        status,
        checked_on: "2026-10-13".to_owned(),
        source: "test".to_owned(),
        finding: "test".to_owned(),
        next_step: "test".to_owned(),
        terms: terms
            .iter()
            .map(|iri| ReadTerm {
                iri: (*iri).to_owned(),
                read_on: "2026-10-13".to_owned(),
                source: "test".to_owned(),
                finding: "test".to_owned(),
            })
            .collect(),
    }
}

fn is_iso_date(text: &str) -> bool {
    let bytes = text.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(i, b)| i == 4 || i == 7 || b.is_ascii_digit())
        && (1..=12).contains(&text[5..7].parse::<u8>().unwrap_or(0))
        && (1..=31).contains(&text[8..10].parse::<u8>().unwrap_or(0))
}

/// The case the ledger exists for: GS1's namespace is verified and one term in it
/// was read, and a string that merely begins with the prefix must not ride on that.
#[test]
fn reading_one_term_does_not_permit_the_rest_of_its_namespace() {
    let register = register();

    assert_eq!(
        register.verdict("https://ref.gs1.org/voc/gtin"),
        Verdict::Permitted {
            vocabulary: "gs1".to_owned()
        }
    );
    let unread = register.verdict("https://ref.gs1.org/voc/Totally_Made_Up_Term");
    assert_eq!(
        unread,
        Verdict::TermNotRead {
            vocabulary: "gs1".to_owned()
        }
    );
    assert!(!unread.is_permitted());
    assert!(unread.reason().contains("nobody has recorded reading"));
}

/// The same holds for every verified vocabulary we may speak, not only GS1: a
/// term nobody listed is refused whatever the namespace.
#[test]
fn every_speakable_verified_vocabulary_refuses_a_term_nobody_listed() {
    let register = register();
    let mut checked = Vec::new();
    for v in register.all().iter().filter(|v| v.permits_emission()) {
        let Some(namespace) = v.namespace_iri.as_deref() else {
            continue;
        };
        let probe = format!("{namespace}aTermNobodyListed");
        assert_eq!(
            register.verdict(&probe),
            Verdict::TermNotRead {
                vocabulary: v.key.clone()
            },
            "'{}' permits a term its record does not list",
            v.key
        );
        checked.push(v.key.as_str());
    }
    checked.sort_unstable();
    assert_eq!(
        checked,
        ["gs1", "idta", "semic"],
        "the speakable verified vocabularies changed; this test must still cover each of them"
    );
}

/// Declaring a prefix names the vocabulary and defines no term, so the JSON-LD
/// context can declare `gs1:` while only `gtin` is read — but only for a
/// vocabulary that is itself permitted.
#[test]
fn a_namespace_iri_is_permitted_exactly_when_its_vocabulary_is() {
    let register = register();

    assert!(register.verdict("https://ref.gs1.org/voc/").is_permitted());
    assert!(
        register
            .verdict("https://admin-shell.io/idta/")
            .is_permitted()
    );
    // Surveyed, tracked and bridge vocabularies keep their own refusals.
    assert!(matches!(
        register.verdict("https://w3id.org/dpv#"),
        Verdict::NotVerified { .. }
    ));
    assert!(matches!(
        register.verdict("https://schema.org/"),
        Verdict::NotVerified { .. }
    ));
    assert!(matches!(
        register.verdict("urn:samm:io.BatteryPass"),
        Verdict::BridgeNotSpeakable { .. }
    ));
}

/// Every entry in the ledger is well-formed and consistent with its record.
#[test]
fn every_listed_term_is_well_formed_and_consistent_with_its_record() {
    let register = register();
    let mut seen = std::collections::HashSet::new();
    let mut listed = 0usize;

    for v in register.all() {
        // A reading recorded under a record that cannot be spoken would look like
        // an entry and permit nothing, which is worse than no entry.
        assert!(
            v.terms.is_empty() || v.permits_emission(),
            "'{}' lists terms but is {:?}/{:?}, so none of them can be emitted",
            v.key,
            v.status,
            v.layer
        );
        for term in &v.terms {
            listed += 1;
            let at = format!("'{}' term {}", v.key, term.iri);

            assert!(seen.insert(term.iri.clone()), "{at} is listed twice");
            assert!(
                !term.iri.chars().any(char::is_whitespace)
                    && (term.iri.contains("://") || term.iri.starts_with("urn:")),
                "{at} is not an absolute IRI"
            );
            assert!(
                !crate::namespace::is_own(&term.iri),
                "{at} is in our own namespace and needs no reading"
            );
            assert!(
                v.namespace_iri.as_deref() != Some(term.iri.as_str()),
                "{at} is the namespace itself, which names the vocabulary and no term"
            );
            for (field, text) in [("source", &term.source), ("finding", &term.finding)] {
                assert!(!text.trim().is_empty(), "{at} has an empty {field}");
            }
            assert!(
                is_iso_date(&term.read_on),
                "{at} has readOn {:?}",
                term.read_on
            );
            assert!(
                term.read_on <= v.checked_on,
                "{at} was read on {}, after the record's own checkedOn {}",
                term.read_on,
                v.checked_on
            );
            // Under its own namespace, or at an address no other record claims.
            // Anything else lets one record vouch for another's namespace.
            let claimed = register.vocabulary_for(&term.iri).map(|c| c.key.as_str());
            assert!(
                claimed.is_none() || claimed == Some(v.key.as_str()),
                "{at} sits under the namespace of '{}', not of '{}'",
                claimed.unwrap_or_default(),
                v.key
            );
        }
    }
    assert!(
        listed >= 1,
        "the ledger lists nothing; the checks above ran on no term"
    );
}

#[test]
fn a_listed_term_outside_the_namespace_is_found_by_its_exact_iri() {
    let register = VocabularyRegister::from_records(vec![record(
        "example",
        Status::Verified,
        Layer::Foundational,
        Some("https://example.invalid/voc/"),
        &["https://elsewhere.invalid/def/Thing"],
    )]);

    assert_eq!(
        register.verdict("https://elsewhere.invalid/def/Thing"),
        Verdict::Permitted {
            vocabulary: "example".to_owned()
        }
    );
    // Neighbours of the listed address are nobody's.
    assert_eq!(
        register.verdict("https://elsewhere.invalid/def/Other"),
        Verdict::Unknown
    );
    assert_eq!(
        register.verdict("https://elsewhere.invalid/def/Thing/more"),
        Verdict::Unknown
    );
}

#[test]
fn a_vocabulary_with_no_namespace_can_still_list_a_term() {
    let register = VocabularyRegister::from_records(vec![record(
        "unnamespaced",
        Status::Verified,
        Layer::Sectoral,
        None,
        &["https://elsewhere.invalid/def/Thing"],
    )]);

    assert!(
        register
            .verdict("https://elsewhere.invalid/def/Thing")
            .is_permitted()
    );
}

/// Listing is not enough on its own: the vocabulary's status and layer still
/// decide, so a stray entry cannot unlock a refused vocabulary.
#[test]
fn a_listed_term_does_not_unlock_an_unverified_or_bridge_vocabulary() {
    let iri = "https://example.invalid/voc/Thing";
    for (status, layer, expected) in [
        (
            Status::Surveyed,
            Layer::Foundational,
            Verdict::NotVerified {
                vocabulary: "example".to_owned(),
                status: Status::Surveyed,
            },
        ),
        (
            Status::Tracked,
            Layer::Foundational,
            Verdict::NotVerified {
                vocabulary: "example".to_owned(),
                status: Status::Tracked,
            },
        ),
        (
            Status::Verified,
            Layer::ConformanceBridge,
            Verdict::BridgeNotSpeakable {
                vocabulary: "example".to_owned(),
            },
        ),
    ] {
        let register = VocabularyRegister::from_records(vec![record(
            "example",
            status,
            layer,
            Some("https://example.invalid/voc/"),
            &[iri],
        )]);
        assert_eq!(register.verdict(iri), expected, "{status:?}/{layer:?}");
    }
}
