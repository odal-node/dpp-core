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
    /// The Art. 8(8) registration identifiers, one per passport, **in
    /// submission order**.
    ///
    /// ✅ COMPLIANCE-PIN: IR (EU) 2026/1778 **Art. 8(8)** — *"the registry shall
    /// generate and store a unique and persistent registration identifier as
    /// part of the registration data"* — and **Art. 8(10)**, which has the
    /// Commission communicate it *"for that specific product … through the user
    /// interface or the API response"*. Per product, so a submission of a
    /// hundred passports yields a hundred of them.
    ///
    /// Empty until the submission reaches [`SubmissionOutcome::Success`]: Art.
    /// 8(8) generates the identifier following successful verification, and a
    /// submission that fails produces no records and therefore no identifiers.
    ///
    /// 🚨 **Correlation is by position, and that is ours.** Nothing published
    /// says how the response ties an identifier to a passport; submission order
    /// is the only correspondence the article's text supports without inventing
    /// a key, and a caller must not read it as the registry's stated contract.
    /// Art. 8(10) also sits awkwardly with 8(8): it communicates the identifier
    /// *"upon successful submission"* while 8(8) generates it *following
    /// successful verification*, which are different moments in an asynchronous
    /// flow — and 8(10) cites paragraph 9 for what paragraph 8 generates. This
    /// models the later moment, because an identifier that verification has not
    /// yet produced is one the registry cannot have sent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub registration_identifiers: Vec<String>,
}
