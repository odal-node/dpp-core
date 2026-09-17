//! Passport status transitions and their refusals.

use super::lifecycle::*;

#[test]
fn valid_transitions() {
    assert!(PassportStatus::Draft.can_transition_to(&PassportStatus::Published));
    assert!(PassportStatus::Draft.can_transition_to(&PassportStatus::Retired));
    assert!(PassportStatus::Published.can_transition_to(&PassportStatus::Suspended));
    assert!(PassportStatus::Published.can_transition_to(&PassportStatus::Retired));
    assert!(PassportStatus::Published.can_transition_to(&PassportStatus::Superseded));
    assert!(PassportStatus::Suspended.can_transition_to(&PassportStatus::Published));
    assert!(PassportStatus::Suspended.can_transition_to(&PassportStatus::Retired));
    assert!(PassportStatus::Published.can_transition_to(&PassportStatus::Deactivated));
    assert!(PassportStatus::Suspended.can_transition_to(&PassportStatus::Deactivated));
}

#[test]
fn invalid_transitions() {
    assert!(!PassportStatus::Draft.can_transition_to(&PassportStatus::Suspended));
    assert!(!PassportStatus::Draft.can_transition_to(&PassportStatus::Superseded));
    assert!(!PassportStatus::Retired.can_transition_to(&PassportStatus::Draft));
    assert!(!PassportStatus::Retired.can_transition_to(&PassportStatus::Published));
    assert!(!PassportStatus::Published.can_transition_to(&PassportStatus::Draft));
    assert!(!PassportStatus::Superseded.can_transition_to(&PassportStatus::Published));
    assert!(!PassportStatus::Superseded.can_transition_to(&PassportStatus::Draft));
    assert!(!PassportStatus::Superseded.can_transition_to(&PassportStatus::Retired));
    // Deactivated is terminal.
    assert!(!PassportStatus::Deactivated.can_transition_to(&PassportStatus::Published));
    assert!(!PassportStatus::Deactivated.can_transition_to(&PassportStatus::Retired));
    // Cannot deactivate a draft or retired record.
    assert!(!PassportStatus::Draft.can_transition_to(&PassportStatus::Deactivated));
    assert!(!PassportStatus::Retired.can_transition_to(&PassportStatus::Deactivated));
}

#[test]
fn superseded_serialises_and_deserialises() {
    let s = serde_json::to_value(PassportStatus::Superseded).unwrap();
    assert_eq!(s.as_str().unwrap(), "superseded");
    let back: PassportStatus = serde_json::from_str("\"superseded\"").unwrap();
    assert_eq!(back, PassportStatus::Superseded);
}

#[test]
fn all_variants_serialise_to_their_wire_string() {
    // Note: Published serialises to "active" (wire compatibility).
    for (status, wire) in [
        (PassportStatus::Draft, "draft"),
        (PassportStatus::Published, "active"),
        (PassportStatus::Suspended, "suspended"),
        (PassportStatus::Retired, "retired"),
        (PassportStatus::Superseded, "superseded"),
        (PassportStatus::Deactivated, "deactivated"),
    ] {
        assert_eq!(
            serde_json::to_value(&status).unwrap().as_str().unwrap(),
            wire
        );
        let back: PassportStatus = serde_json::from_str(&format!("\"{wire}\"")).unwrap();
        assert_eq!(back, status);
    }
}

#[test]
fn published_alias_deserialises_and_unknown_is_rejected() {
    // "published" is accepted as an alias for the "active" wire value.
    let back: PassportStatus = serde_json::from_str("\"published\"").unwrap();
    assert_eq!(back, PassportStatus::Published);

    // An unknown status string is rejected (not silently defaulted).
    assert!(serde_json::from_str::<PassportStatus>("\"bogus\"").is_err());
}

// ── Property tests ────────────────────────────────────────────────────────
use proptest::prelude::*;

fn any_status() -> impl Strategy<Value = PassportStatus> {
    prop_oneof![
        Just(PassportStatus::Draft),
        Just(PassportStatus::Published),
        Just(PassportStatus::Suspended),
        Just(PassportStatus::Retired),
        Just(PassportStatus::Superseded),
        Just(PassportStatus::Deactivated),
    ]
}

proptest! {
    /// Terminal states (Retired, Superseded, Deactivated) have no outgoing
    /// transition to any target — no path resurrects a terminal record.
    #[test]
    fn terminal_states_never_transition_out(to in any_status()) {
        for from in [
            PassportStatus::Retired,
            PassportStatus::Superseded,
            PassportStatus::Deactivated,
        ] {
            prop_assert!(!from.can_transition_to(&to));
        }
    }

    /// Every status round-trips through its JSON wire form.
    #[test]
    fn serde_round_trips(s in any_status()) {
        let json = serde_json::to_string(&s).unwrap();
        let back: PassportStatus = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(s, back);
    }
}

#[test]
fn the_old_archived_wire_value_is_refused_and_says_what_replaced_it() {
    // Not an alias. `"published"` is kept as one for `"active"` because that
    // word means the same thing; `"archived"` is refused because it does not —
    // EN 18221 clause 4.2 uses "archiving" for the retention of historical
    // versions of a *live* passport, and accepting it here would put the
    // ambiguous word back on the wire the rename removed it from.
    let err = serde_json::from_str::<PassportStatus>("\"archived\"")
        .expect_err("`archived` is no longer a status this build accepts");
    let msg = err.to_string();
    assert!(
        msg.contains("`retired`"),
        "the refusal must name the replacement, not just reject: {msg}"
    );
    assert!(
        msg.contains("EN 18221"),
        "and must say why, or it reads as churn: {msg}"
    );
    // The distinction is the point, and a refusal that reads as a deletion
    // teaches the opposite of it: someone told only that `archived` is gone
    // concludes this system does not archive, when clause 4.2 archiving is a
    // live obligation that kept the word. The message has to say both halves.
    assert!(
        msg.contains("not dropped"),
        "the refusal must say the word survives for clause 4.2, or it reads as \
         a removal: {msg}"
    );
}

#[test]
fn no_status_serialises_to_the_vacated_word() {
    // A property rather than a case: whatever variants exist, none may take the
    // word back. This keeps meaning something if a variant is added later.
    for status in PassportStatus::ALL {
        assert_ne!(
            serde_json::to_value(status).unwrap().as_str().unwrap(),
            "archived",
            "{status:?} re-uses the word EN 18221 clause 4.2 needs"
        );
    }
}
