//! Two-phase JAdES construction: derive the signing input, then assemble.
//!
//! # Why two phases
//!
//! The signer is somewhere else. A qualified seal is created by a device we do
//! not hold, reached over a network, and the only operation every provider
//! offers is *sign these bytes*. So construction has to split at exactly that
//! seam: everything before the signature, then everything after it.
//!
//! That shape is also what makes this testable without a provider. The tests
//! sign with a local Ed25519 key; a deployment signs with a QTSP; neither path
//! knows the difference, because the seam is the same.

use base64::Engine;
use sha2::{Digest, Sha256};

use super::header::{JadesError, JadesHeader};

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A JAdES signature awaiting its signature value.
///
/// Holds the two encoded segments and the exact bytes a signer must sign. The
/// segments are retained rather than recomputed so that what is assembled is
/// provably what was signed — recomputing invites a difference, and a
/// difference here is a signature over bytes nobody checked.
#[derive(Debug, Clone)]
pub struct PreparedJades {
    header_b64: String,
    payload_b64: String,
    signing_input: Vec<u8>,
}

impl PreparedJades {
    /// The bytes to sign: `BASE64URL(header) || '.' || BASE64URL(payload)`,
    /// per RFC 7515 clause 5.1 step 5.
    #[must_use]
    pub fn signing_input(&self) -> &[u8] {
        &self.signing_input
    }

    /// SHA-256 of the signing input, for a provider whose API signs a digest
    /// rather than a document.
    ///
    /// This is the hand-off point that makes the provider replaceable: a
    /// "sign this hash" primitive is the one operation every remote signing
    /// service exposes, whereas "produce a JAdES" is a product feature only some
    /// of them sell.
    ///
    /// **The digest algorithm must match what `alg` announces.** This helper
    /// assumes SHA-256, which is right for `ES256`/`RS256`/`PS256` but wrong for
    /// an `alg` built on a different digest — and `EdDSA` over Ed25519 hashes
    /// internally and wants the input itself, not a digest of it. Use
    /// [`Self::signing_input`] when in doubt; it is never wrong.
    #[must_use]
    pub fn signing_input_sha256(&self) -> [u8; 32] {
        Sha256::digest(&self.signing_input).into()
    }

    /// Attach a signature value and produce the compact JAdES.
    ///
    /// `signature` is the raw signature octets as the signing algorithm defines
    /// them — not base64, not DER-wrapped-then-base64. This encodes them.
    #[must_use]
    pub fn assemble(self, signature: &[u8]) -> JadesSignature {
        JadesSignature {
            compact: format!(
                "{}.{}.{}",
                self.header_b64,
                self.payload_b64,
                B64.encode(signature)
            ),
        }
    }
}

/// A complete JAdES signature in JWS compact serialisation.
///
/// Structurally conformant to JAdES-B-B. **Whether it is a qualified electronic
/// seal depends on the certificate and device behind the signature**, not on
/// anything in this type — see the module documentation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JadesSignature {
    compact: String,
}

impl JadesSignature {
    /// The compact serialisation: `header.payload.signature`.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.compact
    }

    /// Consume into the compact string.
    #[must_use]
    pub fn into_string(self) -> String {
        self.compact
    }
}

impl std::fmt::Display for JadesSignature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.compact)
    }
}

/// Prepare a JAdES-B-B signature over `payload` with an attached payload.
///
/// The payload is carried inside the signature, so no `sigD` is emitted and
/// therefore no `crit` — see [`JadesHeader`] for why the standard now
/// deliberately permits that.
///
/// # Errors
///
/// Propagates header construction failure — principally a missing or empty
/// signing-certificate reference, which TS 119 182-1 clause 5.1.7 makes
/// mandatory.
pub fn prepare(header: &JadesHeader, payload: &[u8]) -> Result<PreparedJades, JadesError> {
    let header_b64 = B64.encode(header.to_json_bytes()?);
    let payload_b64 = B64.encode(payload);
    let signing_input = format!("{header_b64}.{payload_b64}").into_bytes();
    Ok(PreparedJades {
        header_b64,
        payload_b64,
        signing_input,
    })
}
