//! [`SubmissionOutcome`] — what became of a submitted request.

use serde::{Deserialize, Serialize};

/// What the registry reports about a submitted registration request.
///
/// 👁️ Observed vocabulary, User Guide v1.02 chapter 6.2.2: requests awaiting
/// validation are listed as `PROCESSING`, and completed ones as `SUCCESS` or
/// `FAILURE`.
///
/// Deliberately **not** merged with
/// [`RegistryStatusCode`](crate::RegistryStatusCode) — see the module
/// documentation for why the two are different questions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum SubmissionOutcome {
    /// Validation has not yet completed. The terminal states are the other two.
    Processing,
    /// Every passport in the submission was accepted.
    Success,
    /// The submission was rejected.
    ///
    /// 🚨 **All of it.** User Guide v1.02: *"When submitting multiple DPPs, if a
    /// single DPP has an error, all the DPPs in the same submission will be
    /// rejected."* There is no partial success, so a caller must not treat a
    /// failure as "some got through" and reconcile the difference.
    Failure,
}

impl SubmissionOutcome {
    /// Whether the registry has finished with this submission.
    ///
    /// A caller polls while this is `false`. It exists so that "is it done?"
    /// has one answer rather than a `match` repeated at every call site, which
    /// is how a new variant becomes a silent fall-through.
    #[must_use]
    pub fn is_terminal(self) -> bool {
        match self {
            Self::Processing => false,
            Self::Success | Self::Failure => true,
        }
    }

    /// The wire value, spelled out rather than derived from `Serialize`.
    ///
    /// Renaming a variant must not silently change what is sent to or matched
    /// against a registry — the same reasoning that governs
    /// [`Granularity::wire_str`](crate::Granularity::wire_str).
    #[must_use]
    pub fn wire_str(self) -> &'static str {
        match self {
            Self::Processing => "PROCESSING",
            Self::Success => "SUCCESS",
            Self::Failure => "FAILURE",
        }
    }
}
