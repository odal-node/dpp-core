//! An envelope of a presentation that was verified.

use serde_json::Value;

/// A `vc+jwt` envelope the caller asked to have
/// [verified](super::EnvelopeDecision::Verify), and that passed every check of
/// [`verify_credential`](super::verify_credential).
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct VerifiedEnvelope {
    /// The envelope's position in `verifiableCredential`.
    pub index: usize,
    /// The credential it carries.
    pub credential: Value,
}
