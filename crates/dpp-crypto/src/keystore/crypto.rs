//! Key derivation and HMAC-integrity helpers for the key store.
//!
//! These are pure functions that derive AES and HMAC keys from a passphrase
//! and compute / verify the envelope HMAC, kept separate from
//! [`super::store`]'s `KeyStore` API.

use aes_gcm::{Aes256Gcm, Key};
use anyhow::{Context, Result};
use argon2::Argon2;
use hmac::{Hmac, Mac};
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::store::{KeyRecord, KeyRecordMap};

type HmacSha256 = Hmac<Sha256>;

/// Derive a 256-bit AES key from a passphrase using Argon2id.
///
/// Parameters follow OWASP recommendations for password hashing:
/// - Memory: 19 MiB (m = 19456)
/// - Iterations: 2
/// - Parallelism: 1
pub(crate) fn derive_aes_key_argon2(passphrase: &str, salt: &[u8]) -> Result<Key<Aes256Gcm>> {
    let params = argon2::Params::new(19456, 2, 1, Some(32))
        .map_err(|e| anyhow::anyhow!("invalid Argon2 params: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut key_bytes = [0u8; 32];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key_bytes)
        .map_err(|e| anyhow::anyhow!("Argon2id key derivation failed: {e}"))?;

    let key = *Key::<Aes256Gcm>::from_slice(&key_bytes);
    key_bytes.zeroize();
    Ok(key)
}

/// Legacy KDF: bare SHA-256 (no salt, no iterations).
/// Only used for reading pre-0.1.0 key stores.
pub(crate) fn derive_aes_key_sha256(passphrase: &str) -> Key<Aes256Gcm> {
    let digest = Sha256::digest(passphrase.as_bytes());
    *Key::<Aes256Gcm>::from_slice(&digest)
}

/// Derive a 32-byte integrity key for HMAC-SHA256 file integrity checks.
///
/// Uses Argon2id with the same salt but a different output length context
/// (64 bytes total, take the last 32). This ensures the integrity key is
/// cryptographically independent from the AES encryption key.
pub(crate) fn derive_integrity_key(passphrase: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let params = argon2::Params::new(19456, 2, 1, Some(64))
        .map_err(|e| anyhow::anyhow!("invalid Argon2 params for integrity key: {e}"))?;
    let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

    let mut key_bytes = [0u8; 64];
    argon2
        .hash_password_into(passphrase.as_bytes(), salt, &mut key_bytes)
        .map_err(|e| anyhow::anyhow!("Argon2id integrity key derivation failed: {e}"))?;

    // Take bytes 32..64 as the integrity key (first 32 would collide
    // with the AES key if params were identical, but the different output
    // length makes them independent).
    let mut integrity_key = [0u8; 32];
    integrity_key.copy_from_slice(&key_bytes[32..64]);
    key_bytes.zeroize();
    Ok(integrity_key)
}

/// Compute HMAC-SHA256 over the full store envelope (`kdf`, `salt`, and `keys`).
///
/// Covering `kdf` and `salt` in the HMAC prevents a downgrade attack where an
/// attacker flips `kdf` to `null`, causing the store to be opened with the
/// legacy SHA-256 path. Without coverage, such a flip would be a DoS
/// (the store becomes unusable) rather than a key extraction, but the HMAC
/// makes even that attack detectable.
///
/// The outer BTreeMap ensures `kdf < keys < salt` field ordering, making the
/// canonical bytes deterministic regardless of runtime map ordering.
pub(crate) fn compute_envelope_hmac(
    integrity_key: &[u8; 32],
    kdf: &str,
    salt_b64: &str,
    keys: &KeyRecordMap,
) -> Result<String> {
    let mac = envelope_mac(integrity_key, kdf, salt_b64, keys)?;
    Ok(hex::encode(mac.finalize().into_bytes()))
}

/// Verify the HMAC-SHA256 of the full envelope against a stored hex-encoded tag.
///
/// The comparison is **constant-time**: the decoded tag is checked via the
/// `hmac` crate's `verify_slice` (backed by `subtle`), so the duration of a
/// failed integrity check does not leak how many tag bytes matched. This is
/// the same constant-time discipline used for API-key and admin-password
/// comparison elsewhere.
pub(crate) fn verify_envelope_hmac(
    integrity_key: &[u8; 32],
    kdf: &str,
    salt_b64: &str,
    keys: &KeyRecordMap,
    stored_hmac_hex: &str,
) -> Result<()> {
    let expected = hex::decode(stored_hmac_hex)
        .context("key store HMAC tag is not valid hex — file may have been tampered with")?;
    let mac = envelope_mac(integrity_key, kdf, salt_b64, keys)?;
    mac.verify_slice(&expected).map_err(|_| {
        anyhow::anyhow!("key store integrity check failed — file may have been tampered with")
    })
}

/// Build the keyed HMAC over the canonical envelope bytes (`kdf`, `salt`,
/// `keys`), ready to be either finalised to a tag or used for constant-time
/// verification. Covering `kdf` and `salt` makes a KDF-downgrade flip detectable.
fn envelope_mac(
    integrity_key: &[u8; 32],
    kdf: &str,
    salt_b64: &str,
    keys: &KeyRecordMap,
) -> Result<HmacSha256> {
    let sorted_keys: std::collections::BTreeMap<&str, &KeyRecord> =
        keys.iter().map(|(k, v)| (k.as_str(), v)).collect();

    let mut outer = std::collections::BTreeMap::new();
    outer.insert(
        "kdf",
        serde_json::to_value(kdf).expect("string is valid JSON"),
    );
    outer.insert(
        "keys",
        serde_json::to_value(&sorted_keys).context("Failed to serialise keys for HMAC")?,
    );
    outer.insert(
        "salt",
        serde_json::to_value(salt_b64).expect("string is valid JSON"),
    );

    let canonical = serde_json::to_vec(&outer).context("Failed to serialise envelope for HMAC")?;
    let mut mac = <HmacSha256 as Mac>::new_from_slice(integrity_key)
        .expect("HMAC key length is always valid");
    mac.update(&canonical);
    Ok(mac)
}
