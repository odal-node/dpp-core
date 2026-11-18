//! What the caller decides about one envelope of a presentation.

use dpp_crypto::jws::PublicKey;

/// The caller's answer for one envelope of a presentation, returned by the closure
/// [`verify_presentation`](super::verify_presentation) calls once per envelope.
///
/// Nothing a presentation envelops passes unverified without being named: an
/// envelope is verified, or it is skipped and reported as skipped, or the
/// presentation is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvelopeDecision {
    /// Verify the envelope under this key. Only a `vc+jwt` can be verified: the
    /// inner token then goes through every check of
    /// [`verify_credential`](super::verify_credential), with the same `now`, and
    /// any failure refuses the presentation. For another media type this refuses the
    /// presentation too, since this crate cannot verify that format.
    Verify(PublicKey),
    /// Do not verify the envelope. The result lists it as skipped.
    Skip,
    /// The caller has no decision, and the presentation is refused. This is the
    /// answer for an envelope whose issuer the caller does not know.
    Unknown,
}
