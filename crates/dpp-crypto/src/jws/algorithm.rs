//! JOSE algorithm constants and allowlist for DPP signatures.

/// JOSE algorithm identifier for Ed25519/EdDSA (RFC 8037 §2).
pub const EDDSA_ALG: &str = "EdDSA";

/// JWK curve name for Ed25519 (RFC 8037 §2).
pub const ED25519_CRV: &str = "Ed25519";

/// The single allowed signing algorithm for all DPP credentials and passport proofs.
///
/// Pinned at compile time so a future algorithm addition requires a deliberate
/// change here plus a corresponding bump of the `algorithm` field in `KeyRecord`.
/// Rejects `alg:none` and all substitution attacks by exhaustive allowlist.
#[inline]
pub fn is_allowed_alg(alg: &str) -> bool {
    alg == EDDSA_ALG
}
