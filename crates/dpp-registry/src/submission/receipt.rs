//! [`SubmissionReceipt`] — the registry's acknowledgement of a submission.

use serde::{Deserialize, Serialize};

use super::{PassportResult, SubmissionOutcome};

/// The registry's acknowledgement of a submission.
///
/// The correlation identifier is the only handle a caller has on a submission
/// whose passports do not yet exist as records, which is every submission until
/// validation completes and all of them if it fails.
///
/// ⚠️ **The field names are ours.** The guide describes a correlation ID, an
/// outcome and a per-passport detail in prose; it does not publish a response
/// schema.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SubmissionReceipt {
    /// The registry's identifier for the submission.
    ///
    /// 👁️ User Guide v1.02: *"Your request will be assigned a correlation ID
    /// and provided a time estimation for the request's validation to be
    /// completed."*
    pub correlation_id: String,
    /// Where the submission has got to.
    pub outcome: SubmissionOutcome,
    /// What the registry reported about each passport, once the submission is
    /// complete.
    ///
    /// Empty while the outcome is
    /// [`Processing`](SubmissionOutcome::Processing): nothing has been ruled on.
    /// On [`Success`](SubmissionOutcome::Success) each passport carries its
    /// Art. 8(8) registration identifier; on
    /// [`Failure`](SubmissionOutcome::Failure) the passports that failed carry
    /// their error code, and none carries an identifier, because a refused
    /// submission produces no records. See [`PassportResult`] for why a result
    /// is matched by identifier and never by position.
    ///
    /// 👁️ In the registry's web client (2026-10-14) these are the task's
    /// `resultPayload.report.itemDetails`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub results: Vec<PassportResult>,
}

impl SubmissionReceipt {
    /// The result for the passport submitted under `unique_product_identifier`,
    /// if the registry reported one.
    ///
    /// `None` is not a verdict. A failure report lists only the passports that
    /// failed, so a passport absent from it was refused with the rest of the
    /// submission and has no error of its own to show.
    #[must_use]
    pub fn result_for(&self, unique_product_identifier: &str) -> Option<&PassportResult> {
        self.results
            .iter()
            .find(|result| result.unique_product_identifier == unique_product_identifier)
    }
}
