//! [`SubmissionOutcome`] — what became of a submitted request.

use serde::{Deserialize, Serialize};

/// What the registry reports about a submitted registration request.
///
/// 👁️ Observed vocabulary, User Guide v1.02 chapter 6.2.2: requests awaiting
/// validation are listed as `PROCESSING`, and completed ones as `SUCCESS` or
/// `FAILURE`.
///
/// 👁️ **Two spellings on the wire**, read from the registry's web client on
/// 2026-10-14. A registration's dashboard task carries a `status` of
/// `PROCESSING`, `SUCCESS` or `FAILURE`, and its `resultPayload.outcomeResult`
/// says `SUCCEEDED` or `FAILED`; the interface shows the latter when it is
/// present and the former otherwise. Both read as the same outcome here, so a
/// caller can take either field without an outcome turning into
/// [`Unrecognised`](Self::Unrecognised) — which is not terminal, and would keep
/// a finished submission polled. A value is written in the `status` spelling.
///
/// Deliberately **not** merged with
/// [`RegistryStatusCode`](crate::RegistryStatusCode) — see the module
/// documentation for why the two are different questions.
///
/// # A value this crate does not know is kept, not refused
///
/// The three names come from what the registry's web interface shows, not from
/// a published API schema, so the API may say something else. A reply carrying
/// an unknown value deserialises as [`Unrecognised`](Self::Unrecognised) with
/// the value intact. A parse error there would be indistinguishable from a
/// broken reply, and a caller holding a queued submission would have no way to
/// tell "the registry said something new" from "the registry said nothing".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
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
    /// A value outside the three above, exactly as the registry sent it.
    ///
    /// **Not terminal.** Nothing says whether an unknown state is final, and
    /// reading it as `Failure` would discard a submission that may have
    /// succeeded, while reading it as `Success` would record a registration
    /// that may not exist. Polling on is the only reading that loses nothing.
    Unrecognised(String),
}

impl SubmissionOutcome {
    /// Whether the registry has finished with this submission.
    ///
    /// A caller polls while this is `false`. It exists so that "is it done?"
    /// has one answer rather than a `match` repeated at every call site, which
    /// is how a new variant becomes a silent fall-through.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        match self {
            Self::Processing | Self::Unrecognised(_) => false,
            Self::Success | Self::Failure => true,
        }
    }

    /// The wire value, spelled out rather than derived from `Serialize`.
    ///
    /// Renaming a variant must not silently change what is sent to or matched
    /// against a registry — the same reasoning that governs
    /// [`Granularity::wire_str`](crate::Granularity::wire_str).
    #[must_use]
    pub fn wire_str(&self) -> &str {
        match self {
            Self::Processing => "PROCESSING",
            Self::Success => "SUCCESS",
            Self::Failure => "FAILURE",
            Self::Unrecognised(value) => value,
        }
    }
}

impl From<String> for SubmissionOutcome {
    fn from(value: String) -> Self {
        match value.as_str() {
            "PROCESSING" => Self::Processing,
            "SUCCESS" | "SUCCEEDED" => Self::Success,
            "FAILURE" | "FAILED" => Self::Failure,
            _ => Self::Unrecognised(value),
        }
    }
}

impl From<SubmissionOutcome> for String {
    fn from(outcome: SubmissionOutcome) -> Self {
        match outcome {
            SubmissionOutcome::Unrecognised(value) => value,
            known => known.wire_str().to_owned(),
        }
    }
}
