//! COSE algorithm identifiers for the four signature algorithms this crate signs
//! and verifies, and the key algorithm each one is.

use crate::jws::KeyAlgorithm;

/// The COSE `alg` value for ES256, ECDSA over P-256 with SHA-256 (RFC 9053 §2.1,
/// Table 1).
pub const ES256_ALG: i64 = -7;

/// The COSE `alg` value for ES384, ECDSA over P-384 with SHA-384 (RFC 9053 §2.1,
/// Table 1).
pub const ES384_ALG: i64 = -35;

/// The COSE `alg` value for ES512, ECDSA over P-521 with SHA-512 (RFC 9053 §2.1,
/// Table 1).
pub const ES512_ALG: i64 = -36;

/// The COSE `alg` value for EdDSA (RFC 9053 §2.2, Table 2), which this crate writes
/// and accepts for an Ed25519 key.
///
/// RFC 9864 §2.2 registers the fully specified `Ed25519` (`-19`) beside it. This
/// crate does not read that value: the W3C VC-JOSE-COSE test suite writes `-8` in
/// every COSE input it signs with an Ed25519 key, and so does this crate, so
/// accepting `-19` is a decision for the day an input needs it. It would open no
/// algorithm-substitution path, because the header is checked against the key.
pub const EDDSA_ALG: i64 = -8;

/// The COSE `alg` value this crate writes for a key of `algorithm`, and the only
/// one it accepts when verifying under such a key.
#[must_use]
pub fn cose_alg(algorithm: KeyAlgorithm) -> i64 {
    match algorithm {
        KeyAlgorithm::Ed25519 => EDDSA_ALG,
        KeyAlgorithm::EcdsaP256 => ES256_ALG,
        KeyAlgorithm::EcdsaP384 => ES384_ALG,
        KeyAlgorithm::EcdsaP521 => ES512_ALG,
    }
}
