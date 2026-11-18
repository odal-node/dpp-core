//! JOSE algorithm constants and allowlist for DPP signatures.

/// The JOSE `alg` identifier this crate **writes** for Ed25519 (RFC 8037 §3.1).
///
/// RFC 9864 deprecates it in favour of [`ED25519_ALG`], and it is still what is
/// written. The Commission's reference validator for AdES signatures recognises
/// only this identifier for JOSE, and so does the key fixture of the W3C
/// VC-JOSE-COSE test suite. RFC 9864 lets a deprecation give way to a documented
/// operational requirement, and interoperating with those is one.
pub const EDDSA_ALG: &str = "EdDSA";

/// The fully specified JOSE `alg` identifier for Ed25519 (RFC 9864).
///
/// Accepted when verifying, never written. It names the same RFC 8032 operation
/// as [`EDDSA_ALG`], so accepting it opens no algorithm-substitution path: the
/// header is checked against the algorithm the key is recorded as using.
pub const ED25519_ALG: &str = "Ed25519";

/// The JOSE `alg` identifier for ECDSA over P-256 with SHA-256 (RFC 7518 §3.4).
pub const ES256_ALG: &str = "ES256";

/// The JOSE `alg` identifier for ECDSA over P-384 with SHA-384 (RFC 7518 §3.4).
pub const ES384_ALG: &str = "ES384";

/// The JOSE `alg` identifier for ECDSA over P-521 with SHA-512 (RFC 7518 §3.4).
pub const ES512_ALG: &str = "ES512";

/// JWK curve name for Ed25519 (RFC 8037 §2).
pub const ED25519_CRV: &str = "Ed25519";

/// JWK curve name for P-256 (RFC 7518 §6.2.1.1).
pub const P256_CRV: &str = "P-256";

/// JWK curve name for P-384 (RFC 7518 §6.2.1.1).
pub const P384_CRV: &str = "P-384";

/// JWK curve name for P-521 (RFC 7518 §6.2.1.1).
pub const P521_CRV: &str = "P-521";

/// The prefix of a JWK Thumbprint URI (RFC 9278), for the SHA-256 hash. What
/// follows it is the RFC 7638 thumbprint.
pub const JWK_THUMBPRINT_URI_PREFIX: &str = "urn:ietf:params:oauth:jwk-thumbprint:sha-256:";

/// Whether `alg` names an algorithm this crate signs or verifies under: the
/// signing algorithms accepted on a JWS for all DPP credentials and passport
/// proofs, `EdDSA` and the RFC 9864 identifier for the same operation, and `ES256`,
/// `ES384` and `ES512`.
///
/// Pinned at compile time, so a future algorithm addition requires a deliberate
/// change in [`KeyAlgorithm::from_jose_alg`] plus a corresponding bump of the
/// `algorithm` field in `KeyRecord`. Membership only: it never selects a
/// verification path. A header's `alg` is checked against the algorithm the *key*
/// is recorded as using, in the verifier's header check. Rejects `alg:none` and
/// all substitution attacks by exhaustive allowlist.
#[inline]
pub fn is_allowed_alg(alg: &str) -> bool {
    KeyAlgorithm::from_jose_alg(alg).is_some()
}

/// The signature algorithm a key pair uses.
///
/// There is one variant per algorithm; [`is_allowed_alg`] admits two spellings of
/// Ed25519 and one each of ES256, ES384 and ES512. The type exists so that the
/// algorithm travels *with* the key rather than being assumed by every reader of it:
/// a verifier can then
/// bind the JWS `alg` header to what the key record says, instead of letting an
/// attacker-supplied header select the verification path.
///
/// `#[non_exhaustive]` so that adding a second algorithm later is not a
/// breaking change for downstream matches.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KeyAlgorithm {
    /// Ed25519 signatures, JOSE `EdDSA` (RFC 8037 §3.1).
    #[serde(rename = "EdDSA")]
    Ed25519,
    /// ECDSA over P-256 with SHA-256, JOSE `ES256` (RFC 7518 §3.4).
    #[serde(rename = "ES256")]
    EcdsaP256,
    /// ECDSA over P-384 with SHA-384, JOSE `ES384` (RFC 7518 §3.4).
    #[serde(rename = "ES384")]
    EcdsaP384,
    /// ECDSA over P-521 with SHA-512, JOSE `ES512` (RFC 7518 §3.4).
    #[serde(rename = "ES512")]
    EcdsaP521,
}

impl KeyAlgorithm {
    /// The JOSE `alg` header value this crate writes.
    #[inline]
    pub fn jose_alg(self) -> &'static str {
        match self {
            Self::Ed25519 => EDDSA_ALG,
            Self::EcdsaP256 => ES256_ALG,
            Self::EcdsaP384 => ES384_ALG,
            Self::EcdsaP521 => ES512_ALG,
        }
    }

    /// The length of a signature this algorithm makes and accepts, in bytes.
    #[inline]
    pub(crate) fn signature_len(self) -> usize {
        match self {
            Self::Ed25519 => 64,
            // `R ‖ S`, each a 32-octet big-endian integer (RFC 7518 §3.4).
            Self::EcdsaP256 => 64,
            // Each a 48-octet big-endian integer (RFC 7518 §3.4).
            Self::EcdsaP384 => 96,
            // Each a 66-octet big-endian integer, 528 bits (RFC 7518 §3.4).
            Self::EcdsaP521 => 132,
        }
    }

    /// Parse a JOSE `alg` header value. `None` for anything not allowed here,
    /// which includes `none`. Both spellings of Ed25519 read as it.
    #[inline]
    pub fn from_jose_alg(alg: &str) -> Option<Self> {
        match alg {
            EDDSA_ALG | ED25519_ALG => Some(Self::Ed25519),
            ES256_ALG => Some(Self::EcdsaP256),
            ES384_ALG => Some(Self::EcdsaP384),
            ES512_ALG => Some(Self::EcdsaP521),
            _ => None,
        }
    }
}
