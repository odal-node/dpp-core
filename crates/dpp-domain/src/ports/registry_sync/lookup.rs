//! [`SubmissionLookup`] — the answer to "which submission is this passport's?".

use serde::{Deserialize, Serialize};

use super::report::SubmissionReport;

/// What looking for a passport's submission found, when the caller no longer
/// holds the correlation identifier it was given.
///
/// The case is
/// [`SubmissionAcknowledgement::AlreadyHeld`](super::SubmissionAcknowledgement::AlreadyHeld):
/// a replayed idempotency key is answered with the conflict and not with the
/// submission, so a caller whose
/// first attempt got through, and whose record of the answer was lost, is left
/// with a submission it cannot poll. The handle can be recovered from the
/// registry's own record of the operator's requests, by the one thing that
/// record reports each passport against: the unique product identifier it was
/// submitted under, never its position in a list.
///
/// A lookup that finds the submission also reads what became of it, so the
/// caller learns the handle and the ruling in one step.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "outcome")]
#[non_exhaustive]
pub enum SubmissionLookup {
    /// The registry's records show this passport's submission.
    #[serde(rename_all = "camelCase")]
    Found {
        /// The handle to poll the submission by from now on.
        correlation_id: String,
        /// What the registry reports about it.
        report: SubmissionReport,
    },
    /// The part of the registry's records that was read shows no such submission.
    ///
    /// Not proof that there is none. A submission still being checked reports no
    /// passport yet, and a record too long to read in full may hold it further
    /// on, which is what `exhaustive` says: `true` only when all of it was read.
    #[serde(rename_all = "camelCase")]
    NotFound {
        /// Whether every record the registry offered was read.
        exhaustive: bool,
    },
    /// More than one submission reports this unique product identifier, and the
    /// lookup will not choose between them.
    ///
    /// An identifier is not unique to a passport: a label keeps resolving to the
    /// newest passport of a product, so a successor and its predecessor can be
    /// submitted under the same one. Picking the first, the last or the nearest
    /// would give a passport another passport's registration, which is worse than
    /// giving it none. The candidates are listed so a person can decide.
    #[serde(rename_all = "camelCase")]
    Ambiguous {
        /// The correlation identifier of each submission that reports it.
        candidates: Vec<String>,
    },
    /// This adapter cannot find a submission without its handle.
    ///
    /// What an adapter answers when it has no way to read the registry's record
    /// of the operator's requests, and what the trait's default answers.
    Unsupported,
}
