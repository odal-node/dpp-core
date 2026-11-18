//! [`KeyEntry::from_private_jwk`]: a signing key built from the private JWK of a key
//! someone else generated, held in memory only.
//!
//! This is not an import into the store. Every key a [`KeyStore`](super::KeyStore)
//! holds was generated there, and that is the promise the store makes, so the entry
//! returned here is the caller's to hold and nothing is written anywhere.

use base64::Engine;
use serde_json::Value;
use zeroize::Zeroizing;

use super::entry::{KeyEntry, SigningMaterial, fingerprint_of};
use super::error::PrivateJwkError;
use crate::jws::algorithm::{ED25519_CRV, KeyAlgorithm, P256_CRV, P384_CRV, P521_CRV};
use crate::jws::public_key::PublicKey;

/// The octets `d` must have: the field size of the curve, which RFC 7518
/// §6.2.2.1 fixes for an EC private key, and the 32-octet seed of an Ed25519 key
/// (RFC 8037 §2).
fn private_len(algorithm: KeyAlgorithm) -> usize {
    match algorithm {
        KeyAlgorithm::Ed25519 | KeyAlgorithm::EcdsaP256 => 32,
        KeyAlgorithm::EcdsaP384 => 48,
        KeyAlgorithm::EcdsaP521 => 66,
    }
}

/// The algorithm a JWK's `kty` and `crv` pair selects: the table
/// `PublicKey::from_jwk` reads.
fn algorithm_of(jwk: &Value) -> Result<KeyAlgorithm, PrivateJwkError> {
    let member = |name: &str| jwk.get(name).and_then(Value::as_str);
    match (member("kty"), member("crv")) {
        (Some("OKP"), Some(ED25519_CRV)) => Ok(KeyAlgorithm::Ed25519),
        (Some("EC"), Some(P256_CRV)) => Ok(KeyAlgorithm::EcdsaP256),
        (Some("EC"), Some(P384_CRV)) => Ok(KeyAlgorithm::EcdsaP384),
        (Some("EC"), Some(P521_CRV)) => Ok(KeyAlgorithm::EcdsaP521),
        _ => Err(PrivateJwkError::UnsupportedKey),
    }
}

/// The octets of the private member `d`, base64url-decoded into a buffer that is
/// zeroized when it drops, including on a refusal, since a partly decoded secret is
/// still secret. The length must be exact. Upstream `from_slice` pads a shorter slice
/// with leading zeros rather than refusing it, so a truncated `d` would otherwise
/// build the same key.
fn decode_private(
    d: &Value,
    algorithm: KeyAlgorithm,
) -> Result<Zeroizing<Vec<u8>>, PrivateJwkError> {
    let encoded = d.as_str().ok_or(PrivateJwkError::PrivateKeyInvalid)?;
    let mut secret = Zeroizing::new(vec![0u8; base64::decoded_len_estimate(encoded.len())]);
    let written = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode_slice(encoded, &mut secret)
        .map_err(|_| PrivateJwkError::PrivateKeyInvalid)?;
    secret.truncate(written);
    if written != private_len(algorithm) {
        return Err(PrivateJwkError::PrivateKeyInvalid);
    }
    Ok(secret)
}

impl KeyEntry {
    /// Build a key pair from the private JWK of a key someone else generated. The key
    /// is held in memory only: it is never written to a [`KeyStore`](super::KeyStore),
    /// which holds only keys it generated.
    ///
    /// `jwk` is a JWK whose `d` member is the private key, such as the `secretKeyJwk`
    /// of a W3C VC-JOSE-COSE test-suite key file. The `kty` and `crv` pair decides the
    /// algorithm: `OKP` with `Ed25519` is Ed25519, and `EC` with `P-256`, `P-384` or
    /// `P-521` is ES256, ES384 or ES512. A declared `alg` must name that algorithm,
    /// where `EdDSA` and `Ed25519` both name Ed25519. `kid`, `use` and `key_ops` are
    /// not read.
    ///
    /// A private JWK carries its public members too (RFC 7518 §6.2.2 for EC,
    /// RFC 8037 §2 for OKP), and they must be the public key that `d` derives.
    /// The constructor refuses, each with its own [`PrivateJwkError`] variant:
    ///
    /// - a JWK with no `d`, which describes a public key;
    /// - a `kty` and `crv` pair that names no supported key, and an `alg` that does not
    ///   name the key's algorithm;
    /// - public members that are missing, of the wrong length, or not a point on the
    ///   curve;
    /// - a `d` that is not base64url, is not the curve's field size (32 octets for
    ///   Ed25519 and P-256, 48 for P-384, 66 for P-521), or is not a valid private key,
    ///   which for an EC curve means zero or at or above the group order;
    /// - a `d` that derives a public key other than the JWK's `x` and `y`.
    ///
    /// The private scalar is zeroized when the entry drops, as a generated key's is.
    /// The JWK belongs to the caller and still holds `d` as text once this returns, so
    /// clearing it is the caller's job.
    ///
    /// # Errors
    /// [`PrivateJwkError`], naming the first rule the JWK breaks.
    pub fn from_private_jwk(jwk: &Value) -> Result<Self, PrivateJwkError> {
        let d = jwk.get("d").ok_or(PrivateJwkError::NotPrivate)?;
        let algorithm = algorithm_of(jwk)?;
        // An absent `alg` is not refused. A present one must be a string naming this key.
        let names_it =
            |alg: &Value| alg.as_str().and_then(KeyAlgorithm::from_jose_alg) == Some(algorithm);
        if !jwk.get("alg").is_none_or(names_it) {
            return Err(PrivateJwkError::AlgorithmMismatch);
        }
        let public = PublicKey::from_jwk(jwk).ok_or(PrivateJwkError::PublicKeyInvalid)?;

        let secret = decode_private(d, algorithm)?;
        let material = SigningMaterial::from_secret(algorithm, &secret)
            .map_err(|_| PrivateJwkError::PrivateKeyInvalid)?;
        if material.public_key() != public {
            return Err(PrivateJwkError::KeyPairMismatch);
        }
        let fingerprint = fingerprint_of(&public);
        Ok(Self::new(material, fingerprint, false))
    }
}
