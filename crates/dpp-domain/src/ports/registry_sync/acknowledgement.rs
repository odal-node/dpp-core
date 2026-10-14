//! [`SubmissionAcknowledgement`] — what the registry says when a passport is submitted.

use serde::{Deserialize, Serialize};

/// The registry's answer to a submission: accepted for validation, never yet a
/// registration.
///
/// IR (EU) 2026/1778 Art. 8(7) has the registry check a submission after it
/// arrives, and Art. 8(8) generates the registration identifier only after that
/// check succeeds. What a submission gets back at once is therefore a handle on
/// the check — a correlation identifier, in the registry's guide — and the
/// verdict is read later with
/// [`check_submission`](super::RegistrySyncPort::check_submission).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
#[non_exhaustive]
pub enum SubmissionAcknowledgement {
    /// The registry accepted the submission and assigned it this handle.
    #[serde(rename_all = "camelCase")]
    Accepted {
        /// The identifier to poll the submission by.
        correlation_id: String,
    },
    /// The registry already holds this submission, and did not say under which
    /// handle.
    ///
    /// What a replayed idempotency key means: an earlier attempt reached the
    /// registry and its answer was lost. Resubmitting would be wrong, and so
    /// would calling it a failure.
    ///
    /// 🚨 **There is nothing to poll.** The registry's reply to a replayed key
    /// identifies the conflict, not the submission, so a caller holding this
    /// has to recover the correlation identifier some other way — the
    /// registry's own record of the operator's requests is the only one known —
    /// and must not invent one or fall back to its own identifiers.
    AlreadyHeld,
}
