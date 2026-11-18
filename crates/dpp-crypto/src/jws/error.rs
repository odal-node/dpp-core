//! Errors of the JWS module.

use super::algorithm::KeyAlgorithm;

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

/// A signature whose length is not the one its algorithm uses: 64 bytes for Ed25519
/// (RFC 8032) and ES256, 96 for ES384 and 132 for ES512 (RFC 7518 §3.4). The verifier
/// returns it through `anyhow`, so a caller can `downcast_ref` it and tell it from a bad
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
