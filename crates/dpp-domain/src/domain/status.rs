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
    /// Every status this build models, for exhaustive iteration.
    ///
    /// `PassportStatus` is `#[non_exhaustive]`, so a consumer outside this crate
    /// cannot enumerate it — and a consumer that publishes an API description
    /// must, in order to list the values its endpoints can return. Without this
    /// the list is hand-written downstream, keeps compiling when a variant is
    /// added here, and the new status ships undocumented: exactly how
    /// `superseded` and `deactivated` came to be missing from the engine's
    /// OpenAPI description while both were reachable.
    ///
    /// A status added later is deliberately not covered until it is added here
    /// on purpose. Same contract as [`crate::domain::seal::SealFormat::ALL`].
    pub const ALL: &'static [Self] = &[
        Self::Draft,
        Self::Published,
        Self::Suspended,
        Self::Archived,
        Self::Superseded,
        Self::Deactivated,
    ];

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
