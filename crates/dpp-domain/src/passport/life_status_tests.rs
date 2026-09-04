//! Every `LifeStatus` variant is reachable, and its wire forms are the Official
//! Journal's own strings.

use super::life_status::LifeStatus;
use crate::disclosure::{Disclosure, PASSPORT_FIELD_DISCLOSURE};

/// `ALL` must list every variant — see `TransferReason`'s equivalent test for
/// why this is two stages rather than one.
#[test]
fn all_lists_every_variant() {
    for status in LifeStatus::ALL {
        match status {
            LifeStatus::Original
            | LifeStatus::Repurposed
            | LifeStatus::Reused
            | LifeStatus::Remanufactured
            | LifeStatus::Waste => {}
        }
    }
    assert_eq!(
        LifeStatus::ALL.len(),
        5,
        "a variant was added to the match above but not to ALL"
    );
}

/// The five wire forms are quoted from Annex XIII point 4(c), verbatim.
///
/// Point 4(c) does not name concepts for us to spell as we like — it enumerates
/// the literal values the status is "defined as". This test is the citation:
/// `re-used` keeps its hyphen, and no sixth value exists. An earlier draft of
/// the design note carried "approaching end of life", which appears nowhere in
/// Regulation (EU) 2023/1542; if that ever comes back, it fails here.
#[test]
fn wire_forms_are_the_annex_xiii_strings() {
    let quoted = [
        (LifeStatus::Original, "original"),
        (LifeStatus::Repurposed, "repurposed"),
        (LifeStatus::Reused, "re-used"),
        (LifeStatus::Remanufactured, "remanufactured"),
        (LifeStatus::Waste, "waste"),
    ];

    assert_eq!(
        quoted.len(),
        LifeStatus::ALL.len(),
        "every status needs its Annex XIII string in this table"
    );

    for (status, expected) in &quoted {
        assert_eq!(
            status.wire_str(),
            *expected,
            "{status:?} must carry the string the annex enumerates"
        );
    }
}

/// The serde form and the hand-written `wire_str` must not diverge.
///
/// `wire_str` is spelled out rather than derived, which is what makes a rename
/// safe — and also what lets the two drift silently if nobody checks. It matters
/// more here than elsewhere: the variants carry explicit `serde(rename)`
/// attributes precisely because the derived camelCase would produce `reused`,
/// which is not a value the instrument contains.
#[test]
fn serde_form_matches_wire_str() {
    for status in LifeStatus::ALL {
        let json = serde_json::to_string(status).expect("status serialises");
        assert_eq!(
            json,
            format!("\"{}\"", status.wire_str()),
            "serde and wire_str disagree for {status:?}"
        );

        let back: LifeStatus =
            serde_json::from_str(&json).expect("status deserialises from its own wire form");
        assert_eq!(back, *status, "{status:?} must survive a round trip");
    }
}

/// The field is Individual, and the passport policy's default is not.
///
/// Point 4's heading restricts it to persons with a legitimate interest, while
/// `ProductGroupAccessPolicy::passport_default` classifies anything unlisted as
/// `Public`. So the entry in `PASSPORT_FIELD_DISCLOSURE` is the only thing
/// standing between an individual unit's life status and an anonymous reader —
/// which makes its absence, not its presence, the silent failure.
#[test]
fn life_status_is_classified_individual() {
    let class = PASSPORT_FIELD_DISCLOSURE
        .iter()
        .find(|(field, _)| *field == "lifeStatus")
        .map(|(_, class)| *class);

    assert_eq!(
        class,
        Some(Disclosure::Individual),
        "lifeStatus must be classified Individual — unlisted means Public, and \
         Annex XIII point 4 is the legitimate-interest tier"
    );
}
