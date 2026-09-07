//! [`SnapshotBound`] — what a continuity snapshot's time bound turned out to be.

use chrono::{DateTime, Utc};

/// The outcome of checking a continuity snapshot's time bound.
///
/// # Why this is not a boolean
///
/// The signatures on an expired snapshot are **fine**. Reporting it as invalid
/// would be misleading — it would send a reader looking for tampering that did
/// not happen — and reporting it as valid would be wrong. There is no third
/// boolean, so there is no boolean.
///
/// The failure this shape exists to prevent is a caller collapsing the cases at
/// its first `if`, which is why there is deliberately **no `is_valid()`**. Two
/// different questions get asked of a snapshot and only the caller can know
/// which one it means:
///
/// - *is a bound claimed here, and does it hold?* — answered by matching.
/// - *may this copy be served?* — depends on where the copy came from, which
///   this crate cannot see. A live read legitimately carries no bound
///   ([`Self::Absent`]); a copy off the static tier that carries none has had
///   its bound stripped, and those two are the same value here.
///
/// So an `is_valid()` would have to guess the caller's question, and whichever
/// it guessed would be silently wrong for the other one.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SnapshotBound {
    /// The document claims no bound.
    ///
    /// This is every live read and every passport signed before snapshots
    /// carried a bound, so it is not a failure. It is also what a stripped
    /// bound looks like — see the type doc.
    Absent,
    /// A bound was proven and has not passed.
    Current {
        /// When the node took this copy.
        as_of: DateTime<Utc>,
        /// When this copy stops being good.
        valid_until: DateTime<Utc>,
    },
    /// A bound was proven and has passed.
    ///
    /// Both signatures verified. The document is intact and stale, and `as_of`
    /// says how stale.
    Expired {
        /// When the node took this copy.
        as_of: DateTime<Utc>,
        /// When this copy stopped being good.
        valid_until: DateTime<Utc>,
    },
    /// A bound is claimed and cannot be trusted.
    ///
    /// The proof is missing, does not verify, covers different bytes, or the
    /// timestamps it covers are unreadable. All of these mean the same thing to
    /// a caller — *something asserted a bound and the assertion does not hold* —
    /// and none of them is expiry.
    Unproven(String),
}

impl SnapshotBound {
    /// True only when a bound was proven and has passed.
    ///
    /// Deliberately narrow: [`Self::Unproven`] is not expired, it is
    /// unverifiable, and a caller that wants to refuse it has to say so.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        matches!(self, Self::Expired { .. })
    }

    /// The bound, when one was actually proven.
    ///
    /// `None` for [`Self::Absent`] and [`Self::Unproven`] — in both, the
    /// document may well carry an `asOf` and a `validUntil`, and in neither has
    /// anything vouched for them. An unverified date on a copy anyone can hold
    /// is not a bound, so it is not returned as one.
    #[must_use]
    pub fn proven(&self) -> Option<(DateTime<Utc>, DateTime<Utc>)> {
        match self {
            Self::Current { as_of, valid_until } | Self::Expired { as_of, valid_until } => {
                Some((*as_of, *valid_until))
            }
            Self::Absent | Self::Unproven(_) => None,
        }
    }
}
