//! [`KeyStore`] — the encrypted on-disk record map, and its persistence envelope.

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng},
};
use anyhow::{Context, Result};
use ed25519_dalek::SigningKey;
use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::crypto::{
    compute_envelope_hmac, derive_aes_key_argon2, derive_aes_key_sha256, derive_integrity_key,
    verify_envelope_hmac,
};
use super::entry::KeyEntry;

/// Type alias for the key-ID → record map stored in the key store.
pub(crate) type KeyRecordMap = HashMap<String, KeyRecord>;

/// Salt length for Argon2id key derivation (16 bytes = 128 bits).
const ARGON2_SALT_LEN: usize = 16;

#[derive(serde::Serialize, serde::Deserialize)]
pub(crate) struct KeyRecord {
    pub(crate) encrypted_signing_key: Vec<u8>,
    pub(crate) nonce: Vec<u8>,
    pub(crate) fingerprint: String,
    pub(crate) verifying_key_hex: String,
    /// True once the key has been revoked (e.g. on compromise). Revoked keys are
    /// excluded from the published DID document, so signatures they produced no
    /// longer verify. Defaults to false (back-compat with pre-revocation stores).
    #[serde(default)]
    pub(crate) revoked: bool,
    /// JOSE algorithm identifier for this key pair (e.g. `"EdDSA"`).
    /// Defaults to `"EdDSA"` for back-compat with pre-algorithm-agility stores.
    #[serde(default = "default_algorithm")]
    pub(crate) algorithm: String,
}

pub(crate) fn default_algorithm() -> String {
    crate::jws::algorithm::EDDSA_ALG.to_owned()
}

/// On-disk envelope for the key store file.
///
/// V2 adds `kdf` and `salt` fields. If `kdf` is missing (V1 format), the
/// store was encrypted with bare SHA-256 and will be transparently migrated
/// to Argon2id on next write.
///
/// V3 adds `hmac` — an HMAC-SHA256 over the serialised `keys` map, keyed
/// with a 32-byte integrity key derived separately from the passphrase.
/// This detects file tampering (swapped keys, modified fingerprints, etc.).
#[derive(serde::Serialize, serde::Deserialize)]
struct StoreEnvelope {
    /// KDF identifier. `"argon2id"` for V2+, absent for V1 (legacy SHA-256).
    #[serde(default)]
    kdf: Option<String>,
    /// Base64-encoded salt used by Argon2id. Absent for V1.
    #[serde(default)]
    salt: Option<String>,
    /// HMAC-SHA256 over the canonical JSON serialisation of `keys`, keyed
    /// with a passphrase-derived integrity key. Absent for V1/V2 stores
    /// (will be added on next write).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hmac: Option<String>,
    /// The key records themselves.
    keys: KeyRecordMap,
}

/// Thread-safe store that loads, encrypts, and caches Ed25519 signing keys.
///
/// Encryption key is derived from a passphrase using Argon2id with a random
/// 128-bit salt. A separate 32-byte integrity key (derived from the same
/// passphrase + salt with a different Argon2 context) is used to compute
/// an HMAC-SHA256 over the serialised key map, protecting against file
/// tampering. Legacy stores (pre-0.1.0) that used bare SHA-256 are
/// automatically migrated on first write.
pub struct KeyStore {
    pub(crate) path: std::path::PathBuf,
    pub(crate) cipher: Aes256Gcm,
    /// 32-byte key used for HMAC-SHA256 file integrity checks.
    pub(crate) integrity_key: [u8; 32],
    pub(crate) salt: [u8; ARGON2_SALT_LEN],
    pub(crate) records: RwLock<KeyRecordMap>,
    /// True if the store was opened with a legacy SHA-256 derived key and
    /// needs re-encryption with Argon2id on next write.
    pub(crate) needs_migration: RwLock<bool>,
}

impl KeyStore {
    pub fn open(path: impl AsRef<Path>, passphrase: &str) -> Result<Self> {
        if path.as_ref().exists() {
            let bytes = std::fs::read(&path).context("Failed to read key store file")?;

            // Try to deserialize as the V2/V3 envelope first. A legacy V0/V1
            // store is a raw `{ "key_id": KeyRecord }` map with no envelope
            // wrapper, so fall back to that shape if the envelope parse fails.
            let envelope: StoreEnvelope = match serde_json::from_slice(&bytes) {
                Ok(env) => env,
                Err(_) => {
                    let keys: KeyRecordMap = serde_json::from_slice(&bytes)
                        .context("Failed to deserialise key store")?;
                    StoreEnvelope {
                        kdf: None,
                        salt: None,
                        hmac: None,
                        keys,
                    }
                }
            };

            if envelope.kdf.as_deref() == Some("argon2id") {
                // V2/V3 format — Argon2id.
                let salt_b64 = envelope.salt.as_deref().ok_or_else(|| {
                    anyhow::anyhow!("key store has kdf=argon2id but no salt field")
                })?;
                let salt_vec =
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, salt_b64)
                        .context("invalid base64 salt in key store")?;
                let salt: [u8; ARGON2_SALT_LEN] = salt_vec.as_slice().try_into().map_err(|_| {
                    anyhow::anyhow!(
                        "key store salt has wrong length: expected {ARGON2_SALT_LEN}, got {}",
                        salt_vec.len()
                    )
                })?;
                let cipher_key = derive_aes_key_argon2(passphrase, &salt)?;
                let cipher = Aes256Gcm::new(&cipher_key);
                let integrity_key = derive_integrity_key(passphrase, &salt)?;

                // Verify HMAC if present (V3). V2 stores without HMAC are
                // accepted — the HMAC will be added on next write.
                if let Some(ref stored_hmac) = envelope.hmac {
                    verify_envelope_hmac(
                        &integrity_key,
                        "argon2id",
                        salt_b64,
                        &envelope.keys,
                        stored_hmac,
                    )?;
                } else {
                    tracing::info!(
                        "key store has no HMAC — integrity check will be added on next write"
                    );
                }

                Ok(Self {
                    path: path.as_ref().to_owned(),
                    cipher,
                    integrity_key,
                    salt,
                    records: RwLock::new(envelope.keys),
                    needs_migration: RwLock::new(false),
                })
            } else {
                // V1 format — legacy SHA-256. Open with legacy KDF, flag for migration.
                tracing::warn!(
                    "key store at {:?} uses legacy SHA-256 KDF — will migrate to Argon2id on next write",
                    path.as_ref()
                );

                // V1 files might be a raw HashMap (pre-envelope) or an
                // envelope with kdf=null. Try the envelope's `keys` first;
                // fall back to treating the whole file as the map.
                let records = if !envelope.keys.is_empty() {
                    envelope.keys
                } else {
                    // Raw V0/V1 format: file is just `{ "key_id": KeyRecord }`.
                    serde_json::from_slice(&bytes)
                        .context("Failed to deserialise legacy key store")?
                };

                let cipher_key = derive_aes_key_sha256(passphrase);
                let cipher = Aes256Gcm::new(&cipher_key);

                // Generate a new salt for the eventual migration.
                let mut salt = [0u8; ARGON2_SALT_LEN];
                OsRng.fill_bytes(&mut salt);

                // Integrity key will be derived properly after migration.
                let integrity_key = derive_integrity_key(passphrase, &salt)?;

                Ok(Self {
                    path: path.as_ref().to_owned(),
                    cipher,
                    integrity_key,
                    salt,
                    records: RwLock::new(records),
                    needs_migration: RwLock::new(true),
                })
            }
        } else {
            // Brand new store — generate a fresh salt.
            let mut salt = [0u8; ARGON2_SALT_LEN];
            OsRng.fill_bytes(&mut salt);
            let cipher_key = derive_aes_key_argon2(passphrase, &salt)?;
            let cipher = Aes256Gcm::new(&cipher_key);
            let integrity_key = derive_integrity_key(passphrase, &salt)?;

            Ok(Self {
                path: path.as_ref().to_owned(),
                cipher,
                integrity_key,
                salt,
                records: RwLock::new(HashMap::new()),
                needs_migration: RwLock::new(false),
            })
        }
    }

    pub fn generate_key(&self, key_id: &str) -> Result<KeyEntry> {
        if *self.needs_migration.read().expect("lock") {
            anyhow::bail!(
                "key store requires KDF migration before writes are allowed — \
                 call migrate_if_needed() first"
            );
        }
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let fingerprint = hex::encode(Sha256::digest(verifying_key.as_bytes()));
        let verifying_key_hex = hex::encode(verifying_key.as_bytes());

        let mut nonce_bytes = [0u8; 12];
        rand::rngs::OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let mut raw = signing_key.to_bytes();
        let encrypted = self
            .cipher
            .encrypt(nonce, raw.as_ref())
            .map_err(|_| anyhow::anyhow!("AES-GCM encrypt failed"))?;
        raw.zeroize();

        let record = KeyRecord {
            encrypted_signing_key: encrypted,
            nonce: nonce_bytes.to_vec(),
            fingerprint: fingerprint.clone(),
            verifying_key_hex: verifying_key_hex.clone(),
            revoked: false,
            algorithm: default_algorithm(),
        };

        {
            let mut map = self.records.write().expect("key store write lock poisoned");
            map.insert(key_id.to_owned(), record);
            self.persist_envelope(&map)?;
        }

        Ok(KeyEntry {
            signing_key,
            verifying_key,
            fingerprint,
            revoked: false,
        })
    }

    pub fn load_key(&self, key_id: &str) -> Result<KeyEntry> {
        let map = self.records.read().expect("key store read lock poisoned");
        let record = map
            .get(key_id)
            .ok_or_else(|| anyhow::anyhow!("no key found for {key_id}"))?;
        self.decrypt_record(record)
    }

    pub fn has_key(&self, key_id: &str) -> bool {
        let map = self.records.read().expect("key store read lock poisoned");
        map.contains_key(key_id)
    }

    /// Return all archived keys for the given identifier in ascending timestamp order.
    pub fn load_archived_keys(&self, key_id: &str) -> Vec<KeyEntry> {
        let prefix = format!("{key_id}#archived-");
        let map = self.records.read().expect("key store read lock poisoned");

        let mut entries: Vec<(&str, &KeyRecord)> = map
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, v)| (k.as_str(), v))
            .collect();

        entries.sort_by_key(|(k, _)| *k);

        let mut result = Vec::with_capacity(entries.len());
        for (key_id, record) in entries {
            match self.decrypt_record(record) {
                Ok(entry) => result.push(entry),
                Err(e) => {
                    tracing::warn!(archive_key = key_id, error = %e, "failed to decrypt archived key — skipping");
                }
            }
        }
        result
    }

    fn decrypt_record(&self, record: &KeyRecord) -> Result<KeyEntry> {
        let nonce = Nonce::from_slice(&record.nonce);
        let mut raw = self
            .cipher
            .decrypt(nonce, record.encrypted_signing_key.as_ref())
            .map_err(|_| anyhow::anyhow!("AES-GCM decrypt failed"))?;

        let bytes: [u8; 32] = raw
            .as_slice()
            .try_into()
            .map_err(|_| anyhow::anyhow!("unexpected key length"))?;
        let signing_key = SigningKey::from_bytes(&bytes);
        let verifying_key = signing_key.verifying_key();
        raw.zeroize();

        Ok(KeyEntry {
            fingerprint: record.fingerprint.clone(),
            signing_key,
            verifying_key,
            revoked: record.revoked,
        })
    }

    pub(crate) fn persist_envelope(&self, map: &KeyRecordMap) -> Result<()> {
        let keys_clone: KeyRecordMap = map
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    KeyRecord {
                        encrypted_signing_key: v.encrypted_signing_key.clone(),
                        nonce: v.nonce.clone(),
                        fingerprint: v.fingerprint.clone(),
                        verifying_key_hex: v.verifying_key_hex.clone(),
                        revoked: v.revoked,
                        algorithm: v.algorithm.clone(),
                    },
                )
            })
            .collect();

        let salt_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.salt);
        let hmac_hex =
            compute_envelope_hmac(&self.integrity_key, "argon2id", &salt_b64, &keys_clone)?;

        let envelope = StoreEnvelope {
            kdf: Some("argon2id".into()),
            salt: Some(salt_b64),
            hmac: Some(hmac_hex),
            keys: keys_clone,
        };
        let bytes = serde_json::to_vec(&envelope).context("Failed to serialise key store")?;
        atomic_write(&self.path, &bytes).context("Failed to write key store file")
    }
}

/// Write `bytes` to `path` atomically: write to a sibling temp file, fsync it,
/// then rename over the target. A crash mid-write therefore leaves the previous
/// key store intact rather than a half-written, integrity-failing file.
fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;

    let dir = path.parent().filter(|p| !p.as_os_str().is_empty());
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("keystore");
    let tmp_name = format!(".{file_name}.tmp.{}", std::process::id());
    let tmp = match dir {
        Some(d) => d.join(tmp_name),
        None => std::path::PathBuf::from(tmp_name),
    };

    let write_result = (|| -> Result<()> {
        let mut f = std::fs::File::create(&tmp).context("create temp key store")?;
        f.write_all(bytes).context("write temp key store")?;
        f.sync_all().context("fsync temp key store")?;
        Ok(())
    })();
    if let Err(e) = write_result {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }

    // `std::fs::rename` replaces an existing destination on both Unix and Windows.
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        anyhow::anyhow!("atomically replace key store: {e}")
    })
}
