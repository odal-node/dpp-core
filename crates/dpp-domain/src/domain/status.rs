//! Passport lifecycle state machine: `PassportStatus` and its valid transitions.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Lifecycle state machine for a Digital Product Passport.
///
/// Valid transitions:
/// ```text
/// Draft      → Published  | Archived
/// Published  → Suspended  | Archived  | Superseded | Deactivated
/// Suspended  → Published  | Archived  | Deactivated
/// ```
/// `Archived`, `Superseded`, and `Deactivated` are terminal — no further
/// transitions. A `Deactivated` passport is retained (the DPP outlives the
/// product, EN 18221) but is end-of-life; the reason lives in the EOL event.
///
/// # Serialisation
/// Serialises to the API wire format: `"draft"`, `"active"`, `"suspended"`,
/// `"archived"`, `"superseded"`, `"deactivated"`. The domain uses `Published`
/// internally; the API and JSON use `"active"`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PassportStatus {
    /// Created but not yet publicly accessible. Default state.
    Draft,
    /// Publicly accessible via QR code. Cryptographically signed.
    Published,
    /// Temporarily hidden from public access (e.g. data dispute, regulatory hold).
    Suspended,
    /// Permanently archived. Immutable. Still accessible for historical queries.
    Archived,
    /// Replaced by a newer passport version. Terminal. The successor passport
    /// carries `supersedes_id` pointing back to this record.
    Superseded,
    /// End-of-life: the product was recycled, destroyed (with a derogation),
    /// exported, or lost. Terminal. The record is retained; the typed reason is
    /// carried by the EOL event (`dpp_domain::domain::eol`). ESPR circularity.
    Deactivated,
}

impl PassportStatus {
    /// The API wire string for this status — shared by [`Serialize`] and
    /// [`std::fmt::Display`] so the two can never drift on the mapping.
    const fn wire_str(&self) -> &'static str {
        match self {
            PassportStatus::Draft => "draft",
            PassportStatus::Published => "active",
            PassportStatus::Suspended => "suspended",
            PassportStatus::Archived => "archived",
            PassportStatus::Superseded => "superseded",
            PassportStatus::Deactivated => "deactivated",
        }
    }
}

impl Serialize for PassportStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.wire_str())
    }
}

impl<'de> Deserialize<'de> for PassportStatus {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "draft" => Ok(PassportStatus::Draft),
            "active" | "published" => Ok(PassportStatus::Published),
            "suspended" => Ok(PassportStatus::Suspended),
            "archived" => Ok(PassportStatus::Archived),
            "superseded" => Ok(PassportStatus::Superseded),
            "deactivated" => Ok(PassportStatus::Deactivated),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &[
                    "draft",
                    "active",
                    "suspended",
                    "archived",
                    "superseded",
                    "deactivated",
                ],
            )),
        }
    }
}

impl PassportStatus {
    /// Returns `true` if transitioning to `next` is a valid state machine transition.
    pub fn can_transition_to(&self, next: &PassportStatus) -> bool {
        use PassportStatus::*;
        matches!(
            (self, next),
            (Draft, Published)
                | (Draft, Archived)
                | (Published, Suspended)
                | (Published, Archived)
                | (Published, Superseded)
                | (Published, Deactivated)
                | (Suspended, Published)
                | (Suspended, Archived)
                | (Suspended, Deactivated)
        )
    }
}

impl std::fmt::Display for PassportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.wire_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
