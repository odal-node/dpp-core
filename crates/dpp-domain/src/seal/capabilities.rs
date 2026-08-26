//! [`SealCapabilities`] — what a seal adapter advertises it can do.

use super::conformance_level::SealConformanceLevel;
use super::envelope::SealEnvelope;
use super::format::SealFormat;
use super::mode::SealMode;
use super::request::SealRequest;

/// Which seal profiles an adapter supports.
///
/// Four axes, mirroring what the CSC API's `credentials/info` reports back —
/// `signature_formats`, `conformance_levels`, and the envelope properties each
/// format admits. Capability discovery is in that protocol already; this is the
/// same idea at the port.
#[derive(Debug, Clone)]
pub struct SealCapabilities {
    pub supported_formats: Vec<SealFormat>,
    pub supported_modes: Vec<SealMode>,
    /// Baseline levels this adapter can produce. An adapter offering only
    /// [`SealConformanceLevel::BaselineB`] cannot seal a retention-locked
    /// document in a way that outlives its own certificate — a procurement
    /// problem, and one a caller can now see rather than discover.
    pub supported_levels: Vec<SealConformanceLevel>,
    /// Packagings this adapter can produce, across every format it supports.
    ///
    /// Flat, and deliberately so: an adapter says which packagings it can build,
    /// and which of those are *legal* for a given format is the protocol's
    /// answer rather than the adapter's ([`SealFormat::envelopes`]). Listing a
    /// packaging here is therefore not a claim that it combines with every
    /// entry in `supported_formats` — [`Self::can_produce`] applies both.
    pub supported_envelopes: Vec<SealEnvelope>,
}

impl SealCapabilities {
    /// Whether these capabilities cover what `req` asks for.
    ///
    /// Defined once, here, so every adapter answers the question the same way.
    /// An adapter that rolled its own check would be free to disagree with the
    /// capabilities it advertises, which is the disagreement the check exists to
    /// make impossible.
    ///
    /// **Every** axis must match, and each carries meaning no other can stand in
    /// for. A provider that produces the right format under the wrong
    /// certificate holder has not produced what was asked for — the mode decides
    /// *whose* attestation the seal is. One that produces the right format at a
    /// lower baseline level has delivered a seal with a shorter life than the
    /// document it covers. Neither is a serialisation detail.
    ///
    /// # The pair check, which is not about this adapter
    ///
    /// The four axes are not independent: a packaging is only meaningful for the
    /// formats that define it ([`SealFormat::envelopes`]). So a request is also
    /// refused when its format and envelope name a pair **no** format defines —
    /// a JAdES seal packaged `Enveloping`, say — regardless of what this adapter
    /// advertises.
    ///
    /// That check belongs here rather than in an adapter because it is not a
    /// statement about any provider. An adapter listing both values separately
    /// would otherwise be read as offering their combination, and the first
    /// place anyone would discover otherwise is a rejected request to a QTSP —
    /// or worse, an attestation packaged some other way.
    pub fn can_produce(&self, req: &SealRequest) -> bool {
        self.supported_formats.contains(&req.sig_format)
            && self.supported_modes.contains(&req.mode)
            && self.supported_levels.contains(&req.conformance_level)
            && self.supported_envelopes.contains(&req.envelope)
            && req.sig_format.admits(req.envelope)
    }

    /// Whether this adapter can produce any seal that outlives its signing
    /// certificate.
    ///
    /// A node whose only sealing provider answers `false` here can issue
    /// passports whose seals stop verifying long before the retention period
    /// ends. Surfaced as a question an operator can ask at boot, rather than one
    /// discovered years later by a verifier.
    #[must_use]
    pub fn can_outlive_certificate_expiry(&self) -> bool {
        self.supported_levels
            .iter()
            .any(|l| l.survives_certificate_expiry())
    }
}
