//! What a verified presentation hands back.

use serde_json::Value;

use super::skipped_envelope::SkippedEnvelope;
use super::verified_envelope::VerifiedEnvelope;

/// A presentation whose outer token verified, with the account of every credential
/// it envelops.
///
/// Each envelope is in exactly one of the two lists, so a caller reads from the
/// result which credentials were verified and which were not looked at. A
/// presentation with an envelope that is in neither was refused instead.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VerifiedPresentation {
    /// The presentation: the verified claims set of the outer token, with the
    /// envelopes as they arrived.
    pub payload: Value,
    /// The envelopes that were verified, in order.
    pub verified: Vec<VerifiedEnvelope>,
    /// The envelopes the caller skipped, in order.
    pub skipped: Vec<SkippedEnvelope>,
}
