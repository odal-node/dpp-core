//! Errors of the JWS module.

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
}
