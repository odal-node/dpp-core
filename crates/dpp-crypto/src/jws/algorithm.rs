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

/// The signature algorithm a key pair uses.
///
/// There is exactly one variant, and [`is_allowed_alg`] still admits only it.
/// The type exists so that the algorithm travels *with* the key rather than
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
    /// The JOSE `alg` header value.
    #[inline]
    pub fn jose_alg(self) -> &'static str {
        match self {
            Self::Ed25519 => EDDSA_ALG,
        }
    }

    /// Parse a JOSE `alg` header value. `None` for anything not allowed here,
    /// which includes `none`.
    #[inline]
    pub fn from_jose_alg(alg: &str) -> Option<Self> {
        match alg {
            EDDSA_ALG => Some(Self::Ed25519),
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
        use base64::Engine;
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        match self {
            Self::Ed25519 => serde_json::json!({
                "kty": "OKP",
                "crv": ED25519_CRV,
                "x": b64.encode(public_key),
            }),
        }
    }
}
