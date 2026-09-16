//! [`SubmissionReceipt`] — the registry's acknowledgement of a submission.

use serde::{Deserialize, Serialize};

use super::SubmissionOutcome;

/// The registry's acknowledgement of a submission.
///
/// The correlation identifier is the only handle a caller has on a submission
/// whose passports do not yet exist as records, which is every submission until
/// validation completes and all of them if it fails.
///
/// ⚠️ **The field names are ours.** The guide describes a correlation ID and an
/// outcome in prose; it does not publish a response schema.
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
}
