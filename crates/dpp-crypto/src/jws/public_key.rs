//! [`PublicKey`]: a public key together with the one algorithm it is used under.
//!
//! Every verification path takes one of these, never raw bytes. The algorithm
//! travels with the key, so a JWS header can only *confirm* it (see
//! the verifier's header check) and never select it. Nothing here infers an
//! algorithm from a length: an Ed25519 signature and an ES256 signature are both
//! 64 bytes, and the ES384 and ES512 signatures are 96 and 132.

use base64::Engine;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::algorithm::{
    ED25519_CRV, JWK_THUMBPRINT_URI_PREFIX, KeyAlgorithm, P256_CRV, P384_CRV, P521_CRV,
};
use super::error::PublicKeyError;

/// The length of an Ed25519 public key in octets, the `x` of its JWK.
const ED25519_KEY_LEN: usize = 32;
/// The length of a P-256 field element, and so of each of `x`, `y`, `R` and `S`.
const P256_FIELD_LEN: usize = 32;
/// The length of a P-384 field element, and so of each of `x`, `y`, `R` and `S`.
const P384_FIELD_LEN: usize = 48;
/// The length of a P-521 field element, and so of each of `x`, `y`, `R` and `S`.
const P521_FIELD_LEN: usize = 66;

/// A validated public key and its algorithm.
///
/// Constructed only through [`Self::from_bytes`] or [`Self::from_jwk`], both of
/// which refuse an encoding that is not a key of that algorithm (an Ed25519
/// encoding that does not decode to a curve point, or an EC encoding that is not
/// a point on its curve). EC keys are held in the SEC1 uncompressed form, whatever
/// form they arrived in, so two encodings of one key compare equal and fingerprint
/// the same. Inside this crate, a key that is already parsed is wrapped without a
/// check.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PublicKey {
    key: Key,
}

/// The parsed key, one variant per algorithm. The variant is the algorithm, so the
/// two cannot disagree. A variant holds the parsed key and any encoding its key
/// type does not keep for itself, so [`PublicKey::as_bytes`] borrows for every
/// algorithm.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Key {
    Ed25519(ed25519_dalek::VerifyingKey),
    /// The parsed P-256 key, with its 65-byte SEC1 uncompressed encoding beside it.
    /// The key's own `Clone`, `Debug`, `PartialEq` and `Eq` make these derives hold.
    EcdsaP256 {
        key: p256::ecdsa::VerifyingKey,
        sec1: Vec<u8>,
    },
    /// The parsed P-384 key, with its 97-byte SEC1 uncompressed encoding beside it.
    EcdsaP384 {
        key: p384::ecdsa::VerifyingKey,
        sec1: Vec<u8>,
    },
    /// The parsed P-521 key, with its 133-byte SEC1 uncompressed encoding beside it.
    EcdsaP521 {
        key: p521::ecdsa::VerifyingKey,
        sec1: Vec<u8>,
    },
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
                let key = ed25519_dalek::VerifyingKey::from_bytes(&array)
                    .map_err(|_| PublicKeyError::Ed25519)?;
                Ok(Self::from_ed25519_key(key))
            }
            KeyAlgorithm::EcdsaP256 => {
                let key = p256::ecdsa::VerifyingKey::from_sec1_bytes(bytes)
                    .map_err(|_| PublicKeyError::P256)?;
                Ok(Self::from_p256_key(key))
            }
            KeyAlgorithm::EcdsaP384 => {
                let key = p384::ecdsa::VerifyingKey::from_sec1_bytes(bytes)
                    .map_err(|_| PublicKeyError::P384)?;
                Ok(Self::from_p384_key(key))
            }
            KeyAlgorithm::EcdsaP521 => {
                let key = p521::ecdsa::VerifyingKey::from_sec1_bytes(bytes)
                    .map_err(|_| PublicKeyError::P521)?;
                Ok(Self::from_p521_key(key))
            }
        }
    }

    /// A public key from a verifying key that is already parsed, so nothing can be
    /// refused: the parse happened when the key was made.
    pub(crate) fn from_ed25519_key(key: ed25519_dalek::VerifyingKey) -> Self {
        Self {
            key: Key::Ed25519(key),
        }
    }

    /// A public key from a P-256 verifying key that is already parsed. Its SEC1
    /// uncompressed encoding is kept beside it, so [`Self::as_bytes`] can borrow.
    pub(crate) fn from_p256_key(key: p256::ecdsa::VerifyingKey) -> Self {
        let sec1 = key.to_sec1_point(false).as_bytes().to_vec();
        Self {
            key: Key::EcdsaP256 { key, sec1 },
        }
    }

    /// A public key from a P-384 verifying key that is already parsed, with its
    /// SEC1 uncompressed encoding kept beside it.
    pub(crate) fn from_p384_key(key: p384::ecdsa::VerifyingKey) -> Self {
        let sec1 = key.to_sec1_point(false).as_bytes().to_vec();
        Self {
            key: Key::EcdsaP384 { key, sec1 },
        }
    }

    /// A public key from a P-521 verifying key that is already parsed, with its
    /// SEC1 uncompressed encoding kept beside it.
    pub(crate) fn from_p521_key(key: p521::ecdsa::VerifyingKey) -> Self {
        let sec1 = key.to_sec1_point(false).as_bytes().to_vec();
        Self {
            key: Key::EcdsaP521 { key, sec1 },
        }
    }

    /// The algorithm this key is used under.
    #[must_use]
    pub fn algorithm(&self) -> KeyAlgorithm {
        match self.key {
            Key::Ed25519(_) => KeyAlgorithm::Ed25519,
            Key::EcdsaP256 { .. } => KeyAlgorithm::EcdsaP256,
            Key::EcdsaP384 { .. } => KeyAlgorithm::EcdsaP384,
            Key::EcdsaP521 { .. } => KeyAlgorithm::EcdsaP521,
        }
    }

    /// The key's bytes: the 32-byte Ed25519 public key, or the SEC1 uncompressed
    /// point for an EC key (65, 97 or 133 bytes for P-256, P-384 and P-521).
    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        match &self.key {
            Key::Ed25519(key) => key.as_bytes(),
            Key::EcdsaP256 { sec1, .. }
            | Key::EcdsaP384 { sec1, .. }
            | Key::EcdsaP521 { sec1, .. } => sec1.as_slice(),
        }
    }

    /// Read a public key from a JWK.
    ///
    /// `OKP` must say `crv: Ed25519`, and the `x` member must be a point on the
    /// curve. `EC` must say `crv` `P-256`, `P-384` or `P-521`, with full-length `x`
    /// and `y` (RFC 7518 §6.2.1.2 and §6.2.1.3 require the full size of a
    /// coordinate), and the point must be on the curve; the curve decides the
    /// algorithm. `None` for any other key type or curve, so a JWK cannot be
    /// mis-selected into a verification path it was not written for.
    #[must_use]
    pub fn from_jwk(jwk: &Value) -> Option<Self> {
        let member = |name: &str| jwk.get(name).and_then(Value::as_str);
        match member("kty")? {
            "OKP" if member("crv")? == ED25519_CRV => {
                let x = member_of_width(jwk, "x", ED25519_KEY_LEN)?;
                Self::from_bytes(KeyAlgorithm::Ed25519, &x).ok()
            }
            "EC" => {
                let (algorithm, field_len) = match member("crv")? {
                    P256_CRV => (KeyAlgorithm::EcdsaP256, P256_FIELD_LEN),
                    P384_CRV => (KeyAlgorithm::EcdsaP384, P384_FIELD_LEN),
                    P521_CRV => (KeyAlgorithm::EcdsaP521, P521_FIELD_LEN),
                    _ => return None,
                };
                let x = member_of_width(jwk, "x", field_len)?;
                let y = member_of_width(jwk, "y", field_len)?;
                if x.len() != field_len || y.len() != field_len {
                    return None;
                }
                let mut sec1 = Vec::with_capacity(1 + 2 * field_len);
                sec1.push(0x04);
                sec1.extend_from_slice(&x);
                sec1.extend_from_slice(&y);
                Self::from_bytes(algorithm, &sec1).ok()
            }
            _ => None,
        }
    }

    /// The JWK members describing this key, for a DID document's `publicKeyJwk`:
    /// its type, its curve and its coordinates. No `kid` and no `alg`.
    ///
    /// The per-algorithm match lives here rather than in the DID-document builder,
    /// so it stays exhaustive in the crate that defines the algorithm. Each further
    /// algorithm needs a `kty` and parameter set that only this crate should have
    /// to know.
    #[must_use]
    pub fn jwk(&self) -> Value {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        match &self.key {
            Key::Ed25519(_) => json!({
                "kty": "OKP",
                "crv": ED25519_CRV,
                "x": b64.encode(self.as_bytes()),
            }),
            Key::EcdsaP256 { sec1, .. } => ec_jwk(P256_CRV, sec1, P256_FIELD_LEN),
            Key::EcdsaP384 { sec1, .. } => ec_jwk(P384_CRV, sec1, P384_FIELD_LEN),
            Key::EcdsaP521 { sec1, .. } => ec_jwk(P521_CRV, sec1, P521_FIELD_LEN),
        }
    }

    /// The RFC 7638 thumbprint: the base64url SHA-256 of the JWK's required
    /// members, ordered by name, with no whitespace.
    ///
    /// The required members of an OKP key are `crv`, `kty` and `x`, and of an EC
    /// key `crv`, `kty`, `x` and `y` (RFC 7638 §3.2). In both, that is also their
    /// order by name. A key's thumbprint is derived from the key alone, so it is
    /// the same before and after a rotation and for every verifier. That is why it,
    /// and not a position in a list, names the key.
    #[must_use]
    pub fn thumbprint(&self) -> String {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let canonical = match &self.key {
            Key::Ed25519(_) => format!(
                r#"{{"crv":"{ED25519_CRV}","kty":"OKP","x":"{}"}}"#,
                b64.encode(self.as_bytes())
            ),
            Key::EcdsaP256 { sec1, .. } => ec_canonical(P256_CRV, sec1, P256_FIELD_LEN),
            Key::EcdsaP384 { sec1, .. } => ec_canonical(P384_CRV, sec1, P384_FIELD_LEN),
            Key::EcdsaP521 { sec1, .. } => ec_canonical(P521_CRV, sec1, P521_FIELD_LEN),
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
                Value::String(self.algorithm().jose_alg().to_owned()),
            );
        }
        jwk
    }

    /// Whether `signature` is this key's signature over `message`, in the form its
    /// algorithm defines.
    ///
    /// - **Ed25519:** 64 bytes, verified strictly. This is this crate's policy, which is
    ///   stricter than RFC 8032 §5.1.7 requires: that section permits the cofactored
    ///   check. Strict verification rejects the signature-malleability and
    ///   small-order/cofactor edge cases that the non-strict `VerifyingKey::verify`
    ///   admits, which is undesirable when the signature is the trust anchor.
    /// - **ES256:** exactly 64 bytes, `R ‖ S` (RFC 7518 §3.4 makes any other length
    ///   a failure), ECDSA over SHA-256. `R` or `S` of zero or not below the group
    ///   order is refused. **A high-`S` signature is accepted**: it is lawful ECDSA,
    ///   other implementations emit it, and the RFC 7515 Appendix A.3 example is
    ///   high-`S`. So an ES256 signature is malleable (`(R, n − S)` verifies too),
    ///   and nothing may use signature bytes as an identifier.
    /// - **ES384** and **ES512:** the same rules over P-384 with SHA-384 and P-521
    ///   with SHA-512, with `R ‖ S` of exactly 96 and 132 bytes. High-`S` is
    ///   accepted for the same reasons.
    #[must_use]
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        match &self.key {
            Key::Ed25519(key) => {
                let Ok(signature_bytes) = <[u8; 64]>::try_from(signature) else {
                    return false;
                };
                key.verify_strict(
                    message,
                    &ed25519_dalek::Signature::from_bytes(&signature_bytes),
                )
                .is_ok()
            }
            Key::EcdsaP256 { key, .. } => {
                use p256::ecdsa::signature::Verifier as _;
                if signature.len() != 2 * P256_FIELD_LEN {
                    return false;
                }
                let Ok(signature) = p256::ecdsa::Signature::from_slice(signature) else {
                    return false;
                };
                key.verify(message, &signature).is_ok()
            }
            Key::EcdsaP384 { key, .. } => {
                use p384::ecdsa::signature::Verifier as _;
                if signature.len() != 2 * P384_FIELD_LEN {
                    return false;
                }
                let Ok(signature) = p384::ecdsa::Signature::from_slice(signature) else {
                    return false;
                };
                key.verify(message, &signature).is_ok()
            }
            Key::EcdsaP521 { key, .. } => {
                use p521::ecdsa::signature::Verifier as _;
                if signature.len() != 2 * P521_FIELD_LEN {
                    return false;
                }
                let Ok(signature) = p521::ecdsa::Signature::from_slice(signature) else {
                    return false;
                };
                key.verify(message, &signature).is_ok()
            }
        }
    }
}

/// The octets of the base64url member `name` of a JWK, when the member is the unpadded
/// base64url of exactly `octets` octets. The encoded length is read before anything is
/// decoded, so an oversized member costs nothing to refuse. `URL_SAFE_NO_PAD` refuses
/// padding and non-zero trailing bits, and of the encoded lengths it accepts only the
/// one given here decodes to `octets`, so this accepts the same members as decoding
/// alone would, minus the oversized ones.
fn member_of_width(jwk: &Value, name: &str, octets: usize) -> Option<Vec<u8>> {
    let encoded = jwk.get(name)?.as_str()?;
    if encoded.len() != base64::encoded_len(octets, false)? {
        return None;
    }
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(encoded)
        .ok()
}

/// `x` and `y` of an EC key held in SEC1 uncompressed form, each `field_len` octets.
fn coordinates(sec1: &[u8], field_len: usize) -> (&[u8], &[u8]) {
    // Construction normalises every EC key to `0x04 ‖ x ‖ y`.
    sec1[1..].split_at(field_len)
}

/// The JWK members of an EC key on `crv`: its type, its curve and its coordinates.
fn ec_jwk(crv: &str, sec1: &[u8], field_len: usize) -> Value {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let (x, y) = coordinates(sec1, field_len);
    json!({
        "kty": "EC",
        "crv": crv,
        "x": b64.encode(x),
        "y": b64.encode(y),
    })
}

/// The RFC 7638 canonical form of an EC key on `crv`: the required members, ordered
/// by name, with no whitespace.
fn ec_canonical(crv: &str, sec1: &[u8], field_len: usize) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let (x, y) = coordinates(sec1, field_len);
    format!(
        r#"{{"crv":"{crv}","kty":"EC","x":"{}","y":"{}"}}"#,
        b64.encode(x),
        b64.encode(y)
    )
}
