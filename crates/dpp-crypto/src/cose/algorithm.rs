//! COSE algorithm identifiers for the four signature algorithms this crate signs
//! and verifies, and the key algorithm each one is.
//!
//! Each key algorithm has two identifiers. RFC 9053 registered a polymorphic one,
//! which does not say which curve it is used with, and RFC 9864 §2 registers a fully
//! specified one that does, and deprecates the polymorphic one (§4.2.2). This crate
//! **writes** the polymorphic value ([`cose_alg`]) and **reads** either
//! ([`accepts_cose_alg`]).
//!
//! | Key | Written (deprecated) | Also read (RFC 9864) |
//! |---|---|---|
//! | Ed25519 | `-8` EdDSA | `-19` Ed25519 |
//! | P-256 | `-7` ES256 | `-9` ESP256 |
//! | P-384 | `-35` ES384 | `-51` ESP384 |
//! | P-521 | `-36` ES512 | `-52` ESP512 |
//!
//! Writing the old value is the exception RFC 9864 §4.4 allows: a replacement SHOULD
//! be used "unless there exist documented operational or regulatory requirements that
//! prevent migration". Here it is the W3C VC-JOSE-COSE test suite, whose COSE inputs
//! carry `-7`, `-35` and `-8`, so a signer that moved first would not be the one the
//! suite expects. `-36` follows them, since the suite has no P-521 COSE input. Reading
//! both costs nothing the key does not already decide, and a verifier that refused
//! `-9` would reject the messages of an issuer that follows the RFC.

use crate::jws::KeyAlgorithm;

/// The COSE `alg` value for ES256, ECDSA over P-256 with SHA-256 (RFC 9053 §2.1,
/// Table 1). RFC 9864 §4.2.2 deprecates it as polymorphic; [`ESP256_ALG`] replaces it.
pub const ES256_ALG: i64 = -7;

/// The COSE `alg` value for ES384, ECDSA over P-384 with SHA-384 (RFC 9053 §2.1,
/// Table 1). RFC 9864 §4.2.2 deprecates it as polymorphic; [`ESP384_ALG`] replaces it.
pub const ES384_ALG: i64 = -35;

/// The COSE `alg` value for ES512, ECDSA over P-521 with SHA-512 (RFC 9053 §2.1,
/// Table 1). RFC 9864 §4.2.2 deprecates it as polymorphic; [`ESP512_ALG`] replaces it.
pub const ES512_ALG: i64 = -36;

/// The COSE `alg` value for EdDSA (RFC 9053 §2.2, Table 2), which this crate writes
/// for an Ed25519 key. RFC 9864 §4.2.2 deprecates it as polymorphic, since it does not
/// say whether the curve is Ed25519 or Ed448; [`ED25519_ALG`] replaces it.
pub const EDDSA_ALG: i64 = -8;

/// The COSE `alg` value for ESP256, ECDSA over P-256 with SHA-256 (RFC 9864 §2.1,
/// Table 1; registered in §4.2.1). The operation is ES256's: the same key, hash and
/// signature encoding, under an identifier that also names the curve.
pub const ESP256_ALG: i64 = -9;

/// The COSE `alg` value for ESP384, ECDSA over P-384 with SHA-384 (RFC 9864 §2.1,
/// Table 1; registered in §4.2.1).
pub const ESP384_ALG: i64 = -51;

/// The COSE `alg` value for ESP512, ECDSA over P-521 with SHA-512 (RFC 9864 §2.1,
/// Table 1; registered in §4.2.1).
pub const ESP512_ALG: i64 = -52;

/// The COSE `alg` value for Ed25519, EdDSA over Curve25519 (RFC 9864 §2.2, Table 2;
/// registered in §4.2.1). The operation is EdDSA's on an Ed25519 key.
pub const ED25519_ALG: i64 = -19;

/// The COSE `alg` value this crate writes for a key of `algorithm`.
///
/// It is the polymorphic value RFC 9864 deprecates, kept for the reason in the module
/// documentation. [`accepts_cose_alg`] decides what is read.
#[must_use]
pub fn cose_alg(algorithm: KeyAlgorithm) -> i64 {
    match algorithm {
        KeyAlgorithm::Ed25519 => EDDSA_ALG,
        KeyAlgorithm::EcdsaP256 => ES256_ALG,
        KeyAlgorithm::EcdsaP384 => ES384_ALG,
        KeyAlgorithm::EcdsaP521 => ES512_ALG,
    }
}

/// The fully specified COSE `alg` value RFC 9864 registers for a key of `algorithm`.
fn fully_specified_cose_alg(algorithm: KeyAlgorithm) -> i64 {
    match algorithm {
        KeyAlgorithm::Ed25519 => ED25519_ALG,
        KeyAlgorithm::EcdsaP256 => ESP256_ALG,
        KeyAlgorithm::EcdsaP384 => ESP384_ALG,
        KeyAlgorithm::EcdsaP521 => ESP512_ALG,
    }
}

/// Whether a protected header's `alg` of `alg` names `algorithm`: the value
/// [`cose_alg`] writes, or its fully specified replacement.
///
/// These are the only two values accepted under a key of `algorithm`. The key decides
/// the algorithm and the header may only confirm it, so a header cannot move a
/// signature to another algorithm: `-9` on a P-384 key, or `-19` on a P-256 one, is a
/// mismatch, as `-35` on a P-256 key is. Accepting the second value opens no
/// algorithm-substitution path: both values name one operation on one key, so the key
/// is still used with a single algorithm (RFC 9864 §7).
#[must_use]
pub fn accepts_cose_alg(algorithm: KeyAlgorithm, alg: i64) -> bool {
    alg == cose_alg(algorithm) || alg == fully_specified_cose_alg(algorithm)
}
