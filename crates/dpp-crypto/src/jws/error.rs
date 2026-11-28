//! Errors of the JWS module.

// LAYOUT-DEVIATION: Rule 14 requires all JWS error types to live in error.rs;
// splitting SignatureLengthError would violate that rule.

use super::algorithm::KeyAlgorithm;
use super::header_refusal::HeaderRefusal;

/// Why bytes were refused as a public key.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PublicKeyError {
    /// Not 32 bytes, or not a valid Ed25519 point encoding.
    #[error("not an Ed25519 public key: 32 bytes encoding a curve point")]
    Ed25519,
    /// Not a SEC1 encoding of a point on P-256.
    #[error("not a P-256 public key: a SEC1 point on the curve")]
    P256,
    /// Not a SEC1 encoding of a point on P-384.
    #[error("not a P-384 public key: a SEC1 point on the curve")]
    P384,
    /// Not a SEC1 encoding of a point on P-521.
    #[error("not a P-521 public key: a SEC1 point on the curve")]
    P521,
}

/// Why a compact JWS could not be verified.
///
/// Each refusal has its own variant so callers can distinguish malformed input,
/// a refused header, and a signature that does not verify.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum JwsError {
    /// The input does not have exactly three compact-serialisation segments.
    #[error("not a compact JWS")]
    MalformedCompact,
    /// The signature segment is not base64url.
    #[error("the signature is not base64url")]
    BadBase64,
    /// The signature length is not the one its algorithm uses.
    #[error(transparent)]
    SignatureLength(#[from] SignatureLengthError),
    /// The protected header is refused for the carried reason.
    #[error("the protected header is refused: {0:?}")]
    HeaderRefused(HeaderRefusal),
    /// The signature is well formed but does not verify under the key.
    #[error("the signature does not verify")]
    SignatureDoesNotVerify,
}

/// A signature whose length is not the one its algorithm uses: 64 bytes for Ed25519
/// (RFC 8032) and ES256, 96 for ES384 and 132 for ES512 (RFC 7518 §3.4). The verifier
/// returns it as [`JwsError::SignatureLength`], so a caller can tell it from a bad
/// signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("{} signature must be {expected} bytes", label(.algorithm))]
#[non_exhaustive]
pub struct SignatureLengthError {
    /// The algorithm the key is used under.
    pub algorithm: KeyAlgorithm,
    /// The length that algorithm's signatures have.
    pub expected: usize,
    /// The length of the signature that was given.
    pub actual: usize,
}

/// The name the wrong-length error gives an algorithm's signatures.
fn label(algorithm: &KeyAlgorithm) -> &'static str {
    match algorithm {
        KeyAlgorithm::Ed25519 => "Ed25519",
        KeyAlgorithm::EcdsaP256 => "ES256",
        KeyAlgorithm::EcdsaP384 => "ES384",
        KeyAlgorithm::EcdsaP521 => "ES512",
    }
}
