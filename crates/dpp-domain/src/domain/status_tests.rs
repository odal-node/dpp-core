//! Passport status transitions and their refusals.

use super::status::*;

#[test]
fn valid_transitions() {
    assert!(PassportStatus::Draft.can_transition_to(&PassportStatus::Published));
    assert!(PassportStatus::Draft.can_transition_to(&PassportStatus::Archived));
    assert!(PassportStatus::Published.can_transition_to(&PassportStatus::Suspended));
    assert!(PassportStatus::Published.can_transition_to(&PassportStatus::Archived));
    assert!(PassportStatus::Published.can_transition_to(&PassportStatus::Superseded));
    assert!(PassportStatus::Suspended.can_transition_to(&PassportStatus::Published));
    assert!(PassportStatus::Suspended.can_transition_to(&PassportStatus::Archived));
    assert!(PassportStatus::Published.can_transition_to(&PassportStatus::Deactivated));
    assert!(PassportStatus::Suspended.can_transition_to(&PassportStatus::Deactivated));
}

#[test]
fn invalid_transitions() {
    assert!(!PassportStatus::Draft.can_transition_to(&PassportStatus::Suspended));
    assert!(!PassportStatus::Draft.can_transition_to(&PassportStatus::Superseded));
    assert!(!PassportStatus::Archived.can_transition_to(&PassportStatus::Draft));
    assert!(!PassportStatus::Archived.can_transition_to(&PassportStatus::Published));
    assert!(!PassportStatus::Published.can_transition_to(&PassportStatus::Draft));
    assert!(!PassportStatus::Superseded.can_transition_to(&PassportStatus::Published));
    assert!(!PassportStatus::Superseded.can_transition_to(&PassportStatus::Draft));
    assert!(!PassportStatus::Superseded.can_transition_to(&PassportStatus::Archived));
    // Deactivated is terminal.
    assert!(!PassportStatus::Deactivated.can_transition_to(&PassportStatus::Published));
    assert!(!PassportStatus::Deactivated.can_transition_to(&PassportStatus::Archived));
    // Cannot deactivate a draft or archived record.
    assert!(!PassportStatus::Draft.can_transition_to(&PassportStatus::Deactivated));
    assert!(!PassportStatus::Archived.can_transition_to(&PassportStatus::Deactivated));
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
        (PassportStatus::Archived, "archived"),
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
        Just(PassportStatus::Archived),
        Just(PassportStatus::Superseded),
        Just(PassportStatus::Deactivated),
    ]
}

proptest! {
    /// Terminal states (Archived, Superseded, Deactivated) have no outgoing
    /// transition to any target — no path resurrects a terminal record.
    #[test]
    fn terminal_states_never_transition_out(to in any_status()) {
        for from in [
            PassportStatus::Archived,
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
