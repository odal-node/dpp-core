//! JOSE algorithm constants and allowlist for DPP signatures.

use base64::Engine;
use sha2::{Digest, Sha256};

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

/// JWK curve name for Ed25519 (RFC 8037 §2).
pub const ED25519_CRV: &str = "Ed25519";

/// The prefix of a JWK Thumbprint URI (RFC 9278), for the SHA-256 hash. What
/// follows it is the RFC 7638 thumbprint.
pub const JWK_THUMBPRINT_URI_PREFIX: &str = "urn:ietf:params:oauth:jwk-thumbprint:sha-256:";

/// The signing algorithms accepted on a JWS for all DPP credentials and passport
/// proofs: `EdDSA`, and the RFC 9864 identifier for the same operation.
///
/// Pinned at compile time so a future algorithm addition requires a deliberate
/// change here plus a corresponding bump of the `algorithm` field in `KeyRecord`.
/// Rejects `alg:none` and all substitution attacks by exhaustive allowlist.
#[inline]
pub fn is_allowed_alg(alg: &str) -> bool {
    alg == EDDSA_ALG || alg == ED25519_ALG
}

/// The signature algorithm a key pair uses.
///
/// There is exactly one variant, and [`is_allowed_alg`] admits two spellings of
/// it. The type exists so that the algorithm travels *with* the key rather than
/// being assumed by every reader of it: a verifier can then bind the JWS `alg`
/// header to what the key record says, instead of letting an attacker-supplied
/// header select the verification path.
///
/// `#[non_exhaustive]` so that adding a second algorithm later is not a
/// breaking change for downstream matches.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum KeyAlgorithm {
    /// Ed25519 signatures, JOSE `EdDSA` (RFC 8037 §3.1).
    #[serde(rename = "EdDSA")]
    Ed25519,
}

impl KeyAlgorithm {
    /// The JOSE `alg` header value this crate writes.
    #[inline]
    pub fn jose_alg(self) -> &'static str {
        match self {
            Self::Ed25519 => EDDSA_ALG,
        }
    }

    /// Parse a JOSE `alg` header value. `None` for anything not allowed here,
    /// which includes `none`. Both spellings of Ed25519 read as it.
    #[inline]
    pub fn from_jose_alg(alg: &str) -> Option<Self> {
        match alg {
            EDDSA_ALG | ED25519_ALG => Some(Self::Ed25519),
            _ => None,
        }
    }

    /// The JWK members describing `public_key`, for a DID document's
    /// `publicKeyJwk`.
    ///
    /// Lives here rather than in the DID-document builder so the per-algorithm
    /// match stays exhaustive in the crate that defines the algorithm. A second
    /// algorithm needs a `kty`/parameter set that only this crate should have
    /// to know — P-256, for instance, is `kty: "EC"` with both `x` and `y`,
    /// where Ed25519 is `kty: "OKP"` with `x` alone.
    pub fn public_key_jwk(self, public_key: &[u8]) -> serde_json::Value {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        match self {
            Self::Ed25519 => serde_json::json!({
                "kty": "OKP",
                "crv": ED25519_CRV,
                "x": b64.encode(public_key),
            }),
        }
    }

    /// The RFC 7638 thumbprint of `public_key`: the base64url SHA-256 of the
    /// JWK's required members, ordered by name, with no whitespace.
    ///
    /// A key's thumbprint is derived from the key alone, so it is the same
    /// before and after a rotation and for every verifier. That is why it, and
    /// not a position in a list, names the key.
    pub fn jwk_thumbprint(self, public_key: &[u8]) -> String {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        match self {
            Self::Ed25519 => {
                // The required members of an OKP key are `crv`, `kty` and `x`,
                // and that is also their order by name.
                let canonical = format!(
                    r#"{{"crv":"{ED25519_CRV}","kty":"OKP","x":"{}"}}"#,
                    b64.encode(public_key)
                );
                b64.encode(Sha256::digest(canonical.as_bytes()))
            }
        }
    }

    /// The RFC 9278 URI form of [`Self::jwk_thumbprint`]. Unlike the bare value
    /// it names its hash, so it reads the same wherever it is carried.
    pub fn thumbprint_uri(self, public_key: &[u8]) -> String {
        format!(
            "{JWK_THUMBPRINT_URI_PREFIX}{}",
            self.jwk_thumbprint(public_key)
        )
    }

    /// The JWK this crate publishes for `public_key`: [`Self::public_key_jwk`],
    /// plus the two members that make it self-describing.
    ///
    /// - `kid` is the thumbprint URI, which Controlled Identifiers recommends as
    ///   the fragment of the verification method's identifier.
    /// - `alg` is the algorithm the key is recorded as using. RFC 7517 says it
    ///   identifies the algorithm the key is intended for, and Controlled
    ///   Identifiers says it should be included to prevent one key being used
    ///   with several algorithms.
    pub fn published_jwk(self, public_key: &[u8]) -> serde_json::Value {
        let mut jwk = self.public_key_jwk(public_key);
        if let Some(object) = jwk.as_object_mut() {
            object.insert(
                "kid".to_owned(),
                serde_json::Value::String(self.thumbprint_uri(public_key)),
            );
            object.insert(
                "alg".to_owned(),
                serde_json::Value::String(self.jose_alg().to_owned()),
            );
        }
        jwk
    }
}
