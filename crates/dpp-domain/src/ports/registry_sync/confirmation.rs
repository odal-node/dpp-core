//! [`SubmissionConfirmation`] — the answer to "is this handle this passport's?".

use serde::{Deserialize, Serialize};

use super::report::SubmissionReport;

/// Whether a correlation identifier someone supplied belongs to a passport's
/// submission.
///
/// The case is the one [`SubmissionLookup`](super::SubmissionLookup) cannot settle:
/// a lookup that finds several submissions reporting a passport's identifier, or
/// none it can place, leaves a person to read the right handle off the
/// registry's own interface. Taking their word for it is how a passport comes to
/// hold another passport's registration, and a typo is enough. So the handle is
/// read back from the registry and checked against what was submitted, by the
/// same unique product identifier a lookup matches on.
///
/// It asks the opposite question to a lookup: not "which submission is this
/// passport's?" but "is this submission this passport's?". The first has an
/// answer only when the registry's records hold exactly one candidate; the second
/// has one whenever the registry will say what the submission contains.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
#[non_exhaustive]
pub enum SubmissionConfirmation {
    /// The submission reports this passport's unique product identifier.
    ///
    /// The report is what the registry says became of it, read in the same step,
    /// so the caller learns that the handle is right and what it leads to
    /// together.
    Confirmed {
        /// What the registry reports about the submission.
        report: SubmissionReport,
    },
    /// The submission was read and does not report this passport's identifier.
    ///
    /// `reported` lists the identifiers it does report, which may be none: a
    /// submission refused as a whole may name no passport at all. Either way
    /// the handle is not this passport's, or cannot be shown to be.
    Mismatch {
        /// The unique product identifiers the submission reports.
        reported: Vec<String>,
    },
    /// The registry has the submission and has not finished checking it.
    ///
    /// A submission still being checked reports no passport yet, so there is
    /// nothing to match against. Not a mismatch: asking again once the registry
    /// has ruled answers it.
    StillChecking,
    /// No submission with that handle was found among the registry's records.
    ///
    /// Not necessarily a mistake in the handle: a record too long to read in
    /// full may hold it further on, which is what `exhaustive` says. `true`
    /// means every record the registry offered was read, so the handle names
    /// nothing.
    #[serde(rename_all = "camelCase")]
    Unlisted {
        /// Whether every record the registry offered was read.
        exhaustive: bool,
    },
    /// This adapter cannot read a submission's contents.
    ///
    /// What an adapter answers when it has no way to see which passports a
    /// submission carries, and what the trait's default answers. A caller must
    /// not read it as a confirmation, nor as a mismatch.
    Unsupported,
}

impl SubmissionConfirmation {
    /// Whether the handle was shown to be the passport's.
    ///
    /// The only answer that licenses a caller to record the handle. Every other
    /// answer, including the ones that are not refusals, leaves it unrecorded.
    #[must_use]
    pub fn is_confirmed(&self) -> bool {
        matches!(self, Self::Confirmed { .. })
    }
}
