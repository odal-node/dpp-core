//! A facility snapshot's typed identifier is additive, and the legacy pair is
//! still read.

use serde_json::json;

use super::FacilitySnapshot;
use crate::identifier::{ActorIdentifier, ActorIdentifierError, Gln, GlnAi, Icd};

const GLN: &str = "9506000134352";
const LEI: &str = "529900T8BM49AURSDO55";

fn snapshot(scheme: &str, value: &str) -> FacilitySnapshot {
    FacilitySnapshot {
        scheme: scheme.into(),
        value: value.into(),
        name: "Plant 3".into(),
        country: "DE".into(),
        address: None,
        identifier: None,
    }
}

/// 🚨 The envelope is additive-only. A snapshot written before the field existed
/// reads, and is written back **without the key**, so its signed bytes do not move.
#[test]
fn a_snapshot_without_the_typed_identifier_reads_and_is_written_back_unchanged() {
    let stored = json!({
        "scheme": "national",
        "value": "DE-BY-0042",
        "name": "Plant 3",
        "country": "DE"
    });
    let read: FacilitySnapshot = serde_json::from_value(stored.clone()).expect("an old snapshot");

    assert_eq!(read.identifier, None);
    assert_eq!(
        serde_json::to_value(&read).expect("serialises"),
        stored,
        "writing an old snapshot back must not add a key"
    );
}

#[test]
fn a_typed_identifier_is_written_and_read_back() {
    let mut facility = snapshot("gln", GLN);
    facility.identifier = Some(ActorIdentifier::gs1(
        GlnAi::Location,
        Gln::parse(GLN).expect("a GLN"),
    ));

    let wire = serde_json::to_value(&facility).expect("serialises");
    assert_eq!(
        wire["identifier"],
        json!({ "scheme": "gs1", "ai": "414", "gln": GLN })
    );
    let read: FacilitySnapshot = serde_json::from_value(wire).expect("reads back");
    assert_eq!(read, facility);
}

/// The typed field is what the snapshot states, and the legacy pair is read only
/// when there is none.
#[test]
fn the_typed_identifier_wins_and_the_legacy_pair_is_read_otherwise() {
    // Legacy only: a `gln` at a facility is a physical location.
    assert_eq!(
        snapshot("gln", GLN).actor_identifier(),
        Ok(ActorIdentifier::gs1(
            GlnAi::Location,
            Gln::parse(GLN).expect("a GLN")
        ))
    );

    // Both: the typed one is stated, whatever the legacy pair says.
    let mut both = snapshot("gln", GLN);
    both.identifier = Some(ActorIdentifier::iso6523(Icd::lei(), LEI).expect("an LEI"));
    assert_eq!(
        both.actor_identifier(),
        ActorIdentifier::iso6523(Icd::lei(), LEI)
    );
}

/// 🚨 A document carrying a scheme with no clause 6 route still reads. Only
/// reading it as a clause 6 identifier says so, and says it is unplaced rather than
/// wrong.
#[test]
fn a_snapshot_with_a_scheme_that_has_no_clause_6_route_still_reads() {
    let national: FacilitySnapshot = serde_json::from_value(json!({
        "scheme": "national", "value": "DE-BY-0042", "name": "Plant 3", "country": "DE"
    }))
    .expect("a national facility code reads");

    assert_eq!(
        national.actor_identifier(),
        Err(ActorIdentifierError::Unmapped {
            scheme: "national".into()
        })
    );
}
