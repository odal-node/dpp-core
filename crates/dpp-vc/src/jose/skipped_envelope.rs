//! An envelope of a presentation that was not verified, because the caller said so.

/// An envelope the caller chose to [skip](super::EnvelopeDecision::Skip), as
/// [`verify_presentation`](super::verify_presentation) reports it.
///
/// The envelope's shape was checked, and its content was not: a skipped
/// credential has not been verified by this crate, whatever its type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SkippedEnvelope {
    /// The envelope's position in `verifiableCredential`.
    pub index: usize,
    /// The media type its `id` names, lower-cased and without parameters:
    /// `application/vc+jwt`, `application/vc+sd-jwt`, `application/vc+cose` and
    /// so on.
    pub media_type: String,
    /// The `kid` the envelope's protected header names, for a `vc+jwt` that has
    /// one.
    pub kid: Option<String>,
}
