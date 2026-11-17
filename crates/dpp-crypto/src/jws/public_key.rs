//! [`PublicKey`]: a public key together with the one algorithm it is used under.
//!
//! Every verification path takes one of these, never raw bytes. The algorithm
//! travels with the key, so a JWS header can only *confirm* it (see
//! the verifier's header check) and never select it.

use base64::Engine;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::algorithm::{ED25519_CRV, JWK_THUMBPRINT_URI_PREFIX, KeyAlgorithm};
use super::error::PublicKeyError;

/// A validated public key and its algorithm.
///
/// Constructed only through [`Self::from_bytes`] or [`Self::from_jwk`], both of
/// which refuse an encoding that is not a key of that algorithm (an Ed25519
/// encoding that does not decode to a curve point).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKey {
    algorithm: KeyAlgorithm,
    bytes: Vec<u8>,
}

impl PublicKey {
    /// Validate `bytes` as a public key of `algorithm`.
    ///
    /// # Errors
    /// [`PublicKeyError`] when the bytes are not a key of that algorithm.
    pub fn from_bytes(algorithm: KeyAlgorithm, bytes: &[u8]) -> Result<Self, PublicKeyError> {
        match algorithm {
            KeyAlgorithm::Ed25519 => {
                let array: [u8; 32] = bytes.try_into().map_err(|_| PublicKeyError::Ed25519)?;
                ed25519_dalek::VerifyingKey::from_bytes(&array)
                    .map_err(|_| PublicKeyError::Ed25519)?;
                Ok(Self {
                    algorithm,
                    bytes: array.to_vec(),
                })
            }
        }
    }

    /// The algorithm this key is used under.
    #[must_use]
    pub fn algorithm(&self) -> KeyAlgorithm {
        self.algorithm
    }

    /// The key's bytes: the 32-byte Ed25519 public key.
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Read a public key from a JWK.
    ///
    /// `OKP` must say `crv: Ed25519`, and the `x` member must be a point on the
    /// curve. `None` for any other key type or curve, so a JWK cannot be
    /// mis-selected into a verification path it was not written for.
    #[must_use]
    pub fn from_jwk(jwk: &Value) -> Option<Self> {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let member = |name: &str| jwk.get(name).and_then(Value::as_str);
        match member("kty")? {
            "OKP" if member("crv")? == ED25519_CRV => {
                let x = b64.decode(member("x")?).ok()?;
                Self::from_bytes(KeyAlgorithm::Ed25519, &x).ok()
            }
            _ => None,
        }
    }

    /// The JWK members describing this key, for a DID document's `publicKeyJwk`:
    /// its type, its curve and its coordinate. No `kid` and no `alg`.
    ///
    /// The per-algorithm match lives here rather than in the DID-document builder,
    /// so it stays exhaustive in the crate that defines the algorithm. A second
    /// algorithm needs a `kty` and parameter set that only this crate should have
    /// to know.
    #[must_use]
    pub fn jwk(&self) -> Value {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        match self.algorithm {
            KeyAlgorithm::Ed25519 => json!({
                "kty": "OKP",
                "crv": ED25519_CRV,
                "x": b64.encode(&self.bytes),
            }),
        }
    }

    /// The RFC 7638 thumbprint: the base64url SHA-256 of the JWK's required
    /// members, ordered by name, with no whitespace.
    ///
    /// The required members of an OKP key are `crv`, `kty` and `x` (RFC 7638
    /// clause 3.2), and that is also their order by name. A key's thumbprint is
    /// derived from the key alone, so it is the same before and after a rotation
    /// and for every verifier. That is why it, and not a position in a list, names
    /// the key.
    #[must_use]
    pub fn thumbprint(&self) -> String {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let canonical = match self.algorithm {
            KeyAlgorithm::Ed25519 => format!(
                r#"{{"crv":"{ED25519_CRV}","kty":"OKP","x":"{}"}}"#,
                b64.encode(&self.bytes)
            ),
        };
        b64.encode(Sha256::digest(canonical.as_bytes()))
    }

    /// The RFC 9278 URI form of [`Self::thumbprint`]. Unlike the bare value it
    /// names its hash, so it reads the same wherever it is carried.
    #[must_use]
    pub fn thumbprint_uri(&self) -> String {
        format!("{JWK_THUMBPRINT_URI_PREFIX}{}", self.thumbprint())
    }

    /// The JWK this crate publishes: [`Self::jwk`] plus the two members that make
    /// it self-describing.
    ///
    /// - `kid` is the thumbprint URI, which Controlled Identifiers recommends as
    ///   the fragment of the verification method's identifier.
    /// - `alg` is the algorithm the key is recorded as using. RFC 7517 says it
    ///   identifies the algorithm the key is intended for, and Controlled
    ///   Identifiers says it should be included to prevent one key being used
    ///   with several algorithms.
    #[must_use]
    pub fn published_jwk(&self) -> Value {
        let mut jwk = self.jwk();
        if let Some(object) = jwk.as_object_mut() {
            object.insert("kid".to_owned(), Value::String(self.thumbprint_uri()));
            object.insert(
                "alg".to_owned(),
                Value::String(self.algorithm.jose_alg().to_owned()),
            );
        }
        jwk
    }

    /// Whether `signature` is this key's signature over `message`: 64 bytes,
    /// verified strictly as RFC 8032 §8 requires. Strict verification rejects the
    /// signature-malleability and small-order/cofactor edge cases that the
    /// non-strict `VerifyingKey::verify` admits, which is undesirable when the
    /// signature is the trust anchor.
    #[must_use]
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        match self.algorithm {
            KeyAlgorithm::Ed25519 => {
                let Ok(key_bytes) = <[u8; 32]>::try_from(self.bytes.as_slice()) else {
                    return false;
                };
                let Ok(signature_bytes) = <[u8; 64]>::try_from(signature) else {
                    return false;
                };
                let Ok(key) = ed25519_dalek::VerifyingKey::from_bytes(&key_bytes) else {
                    return false;
                };
                key.verify_strict(
                    message,
                    &ed25519_dalek::Signature::from_bytes(&signature_bytes),
                )
                .is_ok()
            }
        }
    }
}
