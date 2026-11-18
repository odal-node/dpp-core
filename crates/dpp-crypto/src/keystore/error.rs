//! Errors of the keystore module.

/// Why a JWK was refused as a private key.
///
/// One variant per rule, so a caller and each test can tell the refusals apart. No
/// variant carries a value from the JWK, so an error never prints key material.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PrivateJwkError {
    /// No `d` member: the JWK describes a public key.
    #[error("not a private key: the JWK has no `d` member")]
    NotPrivate,
    /// `kty` and `crv` name no supported key: `OKP` with `Ed25519`, or `EC` with
    /// `P-256`, `P-384` or `P-521`.
    #[error("unsupported key: `kty` and `crv` name no supported key type and curve")]
    UnsupportedKey,
    /// `alg` is present and is not a string that names the key's algorithm.
    #[error("the `alg` member does not name the key's algorithm")]
    AlgorithmMismatch,
    /// The public members `x` (and `y`) are missing, malformed, of the wrong length,
    /// or not a point on the curve.
    #[error("the public members are not a valid public key")]
    PublicKeyInvalid,
    /// `d` is not base64url, is not the curve's field size, or is not a valid private
    /// key. For an EC curve that means zero, or at or above the group order. An Ed25519
    /// `d` is a 32-octet seed (RFC 8037 §2), and any 32 octets are one, so only the
    /// length applies to it.
    #[error("the private member `d` is not a valid private key")]
    PrivateKeyInvalid,
    /// The public key that `d` derives is not the public key in `x` and `y`.
    #[error("the private member `d` does not belong to the JWK's public key")]
    KeyPairMismatch,
}
