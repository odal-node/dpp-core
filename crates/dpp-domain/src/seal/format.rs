//! [`SealFormat`](crate::seal::SealFormat) — the signature format a seal is produced in.

use serde::{Deserialize, Serialize};

use super::envelope::SealEnvelope;

/// AdES signature/seal format family.
///
/// JAdES is the primary format: JSON-native, built on JWS (RFC 7515), and
/// the natural fit for DPP payloads. The others are modelled for completeness.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[non_exhaustive]
pub enum SealFormat {
    /// JSON Advanced Electronic Signatures (ETSI TS 119 182-1) — primary path.
    Jades,
    /// PDF Advanced Electronic Signatures.
    Pades,
    /// CMS Advanced Electronic Signatures (binary/CMS).
    Cades,
    /// XML Advanced Electronic Signatures.
    Xades,
}

impl SealFormat {
    /// Every format this build models, for exhaustive iteration.
    ///
    /// `SealFormat` is `#[non_exhaustive]`, so a consumer outside this crate
    /// cannot enumerate it — and the conformance kit has to, in order to ask an
    /// adapter for a format it does **not** advertise. A format added later is
    /// deliberately not covered until it is added here on purpose.
    pub const ALL: &'static [Self] = &[Self::Jades, Self::Pades, Self::Cades, Self::Xades];

    /// The packagings this format defines, in the CSC API's order.
    ///
    /// Transcribed from the CSC API's `signed_envelope_property` table, where
    /// the permitted values *depend on the value of `signature_format`*. The
    /// sets are not interchangeable and barely overlap:
    ///
    /// | Format | Packagings |
    /// |---|---|
    /// | JAdES | Detached, Attached, Parallel |
    /// | CAdES | Detached, Attached, Parallel |
    /// | XAdES | Enveloped, Enveloping, Detached |
    /// | PAdES | Certification, Revision |
    ///
    /// So `Enveloping` is meaningless for a JAdES seal and `Detached` is
    /// meaningless for a PAdES one — not merely unsupported by some provider,
    /// but undefined by the protocol that would carry the request.
    ///
    /// The first entry of each row is the CSC default for that format. This
    /// deliberately does **not** become the request default: `SealRequest`
    /// defaults to [`SealEnvelope::Detached`] because a request built from a
    /// payload hash means the caller already holds the bytes, and that reasoning
    /// is about our callers rather than about the protocol.
    #[must_use]
    pub const fn envelopes(&self) -> &'static [SealEnvelope] {
        match self {
            Self::Jades | Self::Cades => &[
                SealEnvelope::Attached,
                SealEnvelope::Detached,
                SealEnvelope::Parallel,
            ],
            Self::Xades => &[
                SealEnvelope::Enveloped,
                SealEnvelope::Enveloping,
                SealEnvelope::Detached,
            ],
            Self::Pades => &[SealEnvelope::Certification, SealEnvelope::Revision],
        }
    }

    /// Whether this format defines `envelope` as one of its packagings.
    ///
    /// Asked by [`SealCapabilities::can_produce`](crate::seal::SealCapabilities::can_produce) so that a request naming a
    /// pair no format defines is refused before any adapter sees it.
    #[must_use]
    pub fn admits(&self, envelope: SealEnvelope) -> bool {
        self.envelopes().contains(&envelope)
    }
}
