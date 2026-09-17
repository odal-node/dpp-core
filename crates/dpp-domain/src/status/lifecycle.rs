//! Passport lifecycle state machine: `PassportStatus` and its valid transitions.

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Lifecycle state machine for a Digital Product Passport.
///
/// Valid transitions:
/// ```text
/// Draft      → Published  | Retired
/// Published  → Suspended  | Retired  | Superseded | Deactivated
/// Suspended  → Published  | Retired  | Deactivated
/// ```
/// `Retired`, `Superseded`, and `Deactivated` are terminal — no further
/// transitions. A `Deactivated` passport is retained (the DPP outlives the
/// product, EN 18221) but is end-of-life; the reason lives in the EOL event.
///
/// # Why the terminal state is not called `Archived`
///
/// It was, and the word is EN 18221:2026's, which uses it for something else.
/// That standard's clause 4.2 **archiving** is the retention of historical
/// versions of a passport that is **still live** — a parallel store of past
/// versions, beginning at the first change to the initial passport and kept for
/// the passport's lifetime, with each archived version carrying the same access
/// restrictions as the corresponding current one.
///
/// This variant is none of that. It is a *publication* lifecycle state, reached
/// once the record has stopped changing.
///
/// The collision is between a **status** and a **functionality**, so no doc
/// comment on the variant could remove it: the word was doing two jobs in one
/// domain, and the standard's job is the one a reader arrives with. Anyone
/// mapping this vocabulary onto EN 18221 by name would tick a box that is not
/// ticked. `Retired` keeps the meaning — post-retention, immutable, still
/// readable — and vacates the word.
///
/// **Vacated, not abolished.** Clause 4.2 archiving is a real obligation and the
/// word is the right one for it; what it is not is a status. So "archive",
/// "archiving" and "archived version" remain this domain's vocabulary for the
/// retention of a live passport's historical versions, and are now free to mean
/// only that. A reader who meets either word in this crate outside
/// [`BackupCopyPort`](crate::ports::backup::BackupCopyPort) — which is a third
/// concept again, the ESPR Art. 10(4) back-up copy — should be reading about
/// versions of something still live, never about a state a record is in.
///
/// # Serialisation
/// Serialises to the API wire format: `"draft"`, `"active"`, `"suspended"`,
/// `"retired"`, `"superseded"`, `"deactivated"`. The domain uses `Published`
/// internally; the API and JSON use `"active"`.
///
/// `"archived"` is **not** accepted on the way in. Keeping it as an alias, the
/// way `"published"` is kept for `"active"`, would put the ambiguous word back
/// on the wire — and it is precisely because the word still means something in
/// this domain, just not this, that it cannot also be read as a status. It is
/// refused with a message naming its replacement rather than with a bare
/// unknown-variant error, because a reader meeting that refusal needs the answer
/// and not a list.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum PassportStatus {
    /// Created but not yet publicly accessible. Default state.
    Draft,
    /// Publicly accessible via QR code. Cryptographically signed.
    Published,
    /// Temporarily hidden from public access (e.g. data dispute, regulatory hold).
    Suspended,
    /// Permanently retired. Immutable. Still accessible for historical queries.
    ///
    /// **Not EN 18221 clause 4.2 archiving** — see the type's own note. This is
    /// where a record's publication life ends, not a store of its past versions.
    Retired,
    /// Replaced by a newer passport version. Terminal. The successor passport
    /// carries `supersedes_id` pointing back to this record.
    Superseded,
    /// End-of-life: the product was recycled, destroyed (with a derogation),
    /// exported, or lost. Terminal. The record is retained; the typed reason is
    /// carried by the EOL event (`dpp_domain::eol`). ESPR circularity.
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
    /// on purpose. Same contract as [`crate::seal::SealFormat::ALL`].
    pub const ALL: &'static [Self] = &[
        Self::Draft,
        Self::Published,
        Self::Suspended,
        Self::Retired,
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
            PassportStatus::Retired => "retired",
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
            "retired" => Ok(PassportStatus::Retired),
            "superseded" => Ok(PassportStatus::Superseded),
            "deactivated" => Ok(PassportStatus::Deactivated),
            // Refused deliberately, and answered rather than merely rejected.
            // `unknown_variant` would list the valid set and leave the reader to
            // guess which of them replaced this one.
            "archived" => Err(serde::de::Error::custom(
                "this status is now `retired`. `archived` was not dropped — it \
                 still names what EN 18221 clause 4.2 means by it, the retention \
                 of historical versions of a passport that is still live, which \
                 is a different thing from a terminal lifecycle status. That is \
                 why it is refused here rather than accepted as an alias: one \
                 word cannot carry both meanings on the same wire",
            )),
            other => Err(serde::de::Error::unknown_variant(
                other,
                &[
                    "draft",
                    "active",
                    "suspended",
                    "retired",
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
                | (Draft, Retired)
                | (Published, Suspended)
                | (Published, Retired)
                | (Published, Superseded)
                | (Published, Deactivated)
                | (Suspended, Published)
                | (Suspended, Retired)
                | (Suspended, Deactivated)
        )
    }
}

impl std::fmt::Display for PassportStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.wire_str())
    }
}
