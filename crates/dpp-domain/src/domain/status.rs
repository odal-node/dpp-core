//! Passport lifecycle state machine: `PassportStatus` and its valid transitions.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Lifecycle state machine for a Digital Product Passport.
///
/// Valid transitions:
/// ```text
/// Draft      → Published  | Archived
/// Published  → Suspended  | Archived  | Superseded
/// Suspended  → Published  | Archived
/// ```
/// `Archived` and `Superseded` are terminal — no further transitions.
///
/// # Serialisation
/// Serialises to the API wire format: `"draft"`, `"active"`, `"suspended"`,
/// `"archived"`, `"superseded"`. The domain uses `Published` internally; the
/// API and JSON use `"active"`.
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
}

impl Serialize for PassportStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(match self {
            PassportStatus::Draft => "draft",
            PassportStatus::Published => "active",
            PassportStatus::Suspended => "suspended",
            PassportStatus::Archived => "archived",
            PassportStatus::Superseded => "superseded",
        })
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
            other => Err(serde::de::Error::unknown_variant(
                other,
                &["draft", "active", "suspended", "archived", "superseded"],
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
                | (Suspended, Published)
                | (Suspended, Archived)
        )
    }
}

impl std::fmt::Display for PassportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            PassportStatus::Draft => "draft",
            PassportStatus::Published => "active",
            PassportStatus::Suspended => "suspended",
            PassportStatus::Archived => "archived",
            PassportStatus::Superseded => "superseded",
        };
        write!(f, "{s}")
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
}
