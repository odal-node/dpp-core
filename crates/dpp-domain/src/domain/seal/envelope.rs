//! [`SealEnvelope`] — how a signature is packaged relative to its payload.

use serde::{Deserialize, Serialize};

/// Where the seal sits relative to the bytes it covers.
///
/// # Why the port models both rather than choosing
///
/// A qualified seal **can** be a JWS signature: JAdES (ETSI TS 119 182-1) is an
/// AdES format built on RFC 7515, and its scope covers qualified electronic
/// seals explicitly. So "the seal is the payload's signature" and "the seal
/// wraps a digest of the payload" are both available at the standards layer, and
/// JAdES itself spans them — its `sigD` header identifies a detached payload
/// precisely because RFC 7515 has no native way to.
///
/// What decides which is available is **the provider's format menu, not the
/// law**. Those move independently, so a port that picked one would encode a
/// supplier's product decision as an architectural one. It is a capability.
///
/// # Not every packaging goes with every format
///
/// The values below are the union across all formats, and the union is not
/// meaningful on its own — the CSC API defines `signed_envelope_property` **per
/// signature format**, and the sets barely overlap. `Enveloping` is an XAdES
/// packaging; `Certification` and `Revision` are PAdES revisions and mean
/// nothing elsewhere. Which ones a given format admits is
/// [`SealFormat::envelopes`], and [`SealCapabilities::can_produce`] rejects a
/// pair no format defines.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
#[non_exhaustive]
pub enum SealEnvelope {
    /// The seal is returned alongside the data, covering a digest of it.
    /// Two artefacts: the payload signs itself, the seal attests it separately.
    Detached,
    /// The signed data is carried inside the seal structure. One artefact.
    Enveloping,
    /// The seal is embedded into the document it covers, which stays readable
    /// in its own format.
    Attached,
    /// Independent seals over the same content, none countersigning another.
    ///
    /// The packaging for *two parties attesting the same passport* — a
    /// manufacturer's seal and a third party's beside it — which is why its
    /// absence mattered enough to add it.
    Parallel,
    /// The seal is placed inside the signed document's own structure, which
    /// continues to contain the data. XAdES only.
    Enveloped,
    /// A PAdES certification (author) signature: the first, which may restrict
    /// what later revisions are permitted to change.
    Certification,
    /// A PAdES approval signature in an incremental revision of the document.
    Revision,
}

impl SealEnvelope {
    /// Every packaging this build models. Same reasoning as [`SealFormat::ALL`].
    ///
    /// The union across formats. To enumerate the packagings that are legal for
    /// one format, use [`SealFormat::envelopes`].
    pub const ALL: &'static [Self] = &[
        Self::Detached,
        Self::Enveloping,
        Self::Attached,
        Self::Parallel,
        Self::Enveloped,
        Self::Certification,
        Self::Revision,
    ];
}
