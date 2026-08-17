//! [`KeyStore`] — the encrypted on-disk record map, and its persistence envelope.

use std::collections::HashMap;
use std::path::Path;
use std::sync::RwLock;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload, consts::U12},
};
use anyhow::{Context, Result};
use ed25519_dalek::SigningKey;
use rand::Rng;
use sha2::{Digest, Sha256};
use zeroize::{Zeroize, Zeroizing};

use super::crypto::{
    compute_envelope_hmac, derive_aes_key_argon2, derive_aes_key_sha256, derive_integrity_key,
    verify_envelope_hmac,
};
use super::entry::KeyEntry;
use crate::jws::algorithm::KeyAlgorithm;

/// Type alias for the key-ID → record map stored in the key store.
pub(crate) type KeyRecordMap = HashMap<String, KeyRecord>;

/// Salt length for Argon2id key derivation (16 bytes = 128 bits).
const ARGON2_SALT_LEN: usize = 16;

/// The store format this build writes.
///
/// V1 was a bare record map under a SHA-256-derived key. V2 added `kdf`/`salt`
/// (Argon2id). V3 added the envelope `hmac`. **V4 binds each record's ciphertext
/// to its own fingerprint** via AES-GCM associated data — see
/// [`KeyStore::record_aad`].
///
/// Absent from a file means pre-V4, which is the only thing the number is used
/// to decide.
const STORE_VERSION: u32 = 4;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
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
    /// The signature algorithm this key pair uses. Serialises as its JOSE
    /// identifier (`"EdDSA"`), so the on-disk shape is unchanged. Defaults for
    /// back-compat with pre-algorithm-agility stores; an *unrecognised*
    /// algorithm fails to deserialise rather than loading a key nothing can
    /// safely use.
    #[serde(default = "default_algorithm")]
    pub(crate) algorithm: KeyAlgorithm,
}

impl KeyRecord {
    /// Construct a fresh, non-revoked record for a newly generated key pair.
    pub(crate) fn new(
        encrypted_signing_key: Vec<u8>,
        nonce: Vec<u8>,
        fingerprint: String,
        verifying_key_hex: String,
    ) -> Self {
        Self {
            encrypted_signing_key,
            nonce,
            fingerprint,
            verifying_key_hex,
            revoked: false,
            algorithm: default_algorithm(),
        }
    }
}

pub(crate) fn default_algorithm() -> KeyAlgorithm {
    KeyAlgorithm::Ed25519
}

/// A key's public half plus its revocation state, read directly from the
/// stored record's plaintext `verifying_key_hex`/`revoked` fields — no
/// private-key decryption involved. For callers (like the `did:web` document
/// builder in `dpp-vc`) that only ever need the public key, this avoids an
/// AES-GCM decrypt per key on every call.
///
/// `algorithm` travels with the key because a reader cannot otherwise know how
/// to represent it: the DID-document builder needs it to choose the JWK shape,
/// and guessing is how a key ends up published under the wrong `kty`.
#[non_exhaustive]
pub struct PublicKeyInfo {
    pub verifying_key_hex: String,
    pub revoked: bool,
    pub algorithm: KeyAlgorithm,
}

impl From<&KeyRecord> for PublicKeyInfo {
    fn from(record: &KeyRecord) -> Self {
        Self {
            verifying_key_hex: record.verifying_key_hex.clone(),
            revoked: record.revoked,
            algorithm: record.algorithm,
        }
    }
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
    /// Store format version. Absent means pre-V4 — records are not bound to
    /// their fingerprints. See [`STORE_VERSION`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    version: Option<u32>,
    /// KDF identifier. `"argon2id"` for V2+, absent for V1 (legacy SHA-256).
    #[serde(default)]
    kdf: Option<String>,
    /// Base64-encoded salt used by Argon2id. Absent for V1.
    #[serde(default)]
    salt: Option<String>,
    /// HMAC-SHA256 over the canonical JSON serialisation of `keys`, keyed
    /// with a passphrase-derived integrity key. Absent for V1/V2 stores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    hmac: Option<String>,
    /// The key records themselves.
    keys: KeyRecordMap,
}

/// Why a store cannot be opened by [`KeyStore::open`] without being upgraded.
///
/// Each variant is a security property the store predates. They are reported
/// rather than silently accommodated: every one of them was previously accepted
/// on open, which meant a store could be *downgraded* into the weaker shape and
/// opened without complaint.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpgradeNeeded {
    /// Encrypted under the bare SHA-256 KDF — no salt, no iterations.
    LegacyKdf,
    /// No envelope HMAC, so the plaintext fields — including `revoked` — are
    /// unauthenticated and a record swap is undetectable.
    MissingIntegrityTag,
    /// Records are not bound to their fingerprints (pre-V4).
    UnboundRecords,
}

impl UpgradeNeeded {
    /// What is wrong, and what it costs.
    const fn describe(self) -> &'static str {
        match self {
            Self::LegacyKdf => "encrypted with the legacy SHA-256 KDF (no salt, no iterations)",
            Self::MissingIntegrityTag => {
                "carries no integrity tag, so its revocation flags and key IDs are unauthenticated"
            }
            Self::UnboundRecords => {
                "predates per-record binding, so a record's ciphertext is not tied to its own fingerprint"
            }
        }
    }
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
    /// Which security property this store predates, if any. `None` once it is
    /// at [`STORE_VERSION`] with an Argon2id key and a verified integrity tag.
    pub(crate) upgrade_needed: Option<UpgradeNeeded>,
    /// Whether this store's records are bound to their own fingerprints.
    ///
    /// Read on every decrypt: a pre-V4 record was encrypted with no associated
    /// data and will not open if we supply some.
    pub(crate) records_bound: bool,
}

impl KeyStore {
    /// Open a store, refusing one that predates any of the current security
    /// properties.
    ///
    /// # Why this refuses rather than accommodates
    ///
    /// Every legacy shape [`UpgradeNeeded`] names was previously accepted here
    /// silently: a store with `kdf` absent opened under the unsalted SHA-256
    /// KDF, and a store with no `hmac` opened with **no integrity check at
    /// all** — which left the plaintext `revoked` flag unauthenticated, and that
    /// flag is what `dpp-vc`'s `did:web` builder reads to drop a compromised key
    /// from the published DID document.
    ///
    /// Accepting a weaker shape on read means an attacker who can write the file
    /// can *choose* the weaker shape. The tolerance was for stores written by
    /// older versions of this crate, and refusing them here does not strand one:
    /// [`Self::open_and_migrate`] upgrades and opens them. What changes is that
    /// the weak path now requires a caller who asked for it by name.
    ///
    /// # Errors
    ///
    /// Names the specific property the store predates, and points at
    /// `open_and_migrate`.
    pub fn open(path: impl AsRef<Path>, passphrase: &str) -> Result<Self> {
        let store = Self::open_permissively(path, passphrase)?;
        if let Some(reason) = store.upgrade_needed {
            anyhow::bail!(
                "key store {}; open it with `KeyStore::open_and_migrate` to upgrade it in place",
                reason.describe()
            );
        }
        Ok(store)
    }

    /// Open a store whatever shape it is in, recording what it predates.
    ///
    /// `pub(crate)` — [`Self::open`] and [`Self::open_and_migrate`] are the two
    /// doors, and this is deliberately not a third.
    pub(crate) fn open_permissively(path: impl AsRef<Path>, passphrase: &str) -> Result<Self> {
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
                        version: None,
                        kdf: None,
                        salt: None,
                        hmac: None,
                        keys,
                    }
                }
            };
            let records_bound = envelope.version.unwrap_or(0) >= STORE_VERSION;

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

                // A present HMAC is always verified, and a failure is fatal
                // regardless of which door was used: a tampered store is not a
                // store to be upgraded, it is one to be refused.
                //
                // An absent HMAC no longer opens quietly. It is recorded as an
                // upgrade requirement so `open` refuses it and
                // `open_and_migrate` repairs it — the plaintext `revoked` flag
                // is unauthenticated without it, and that flag decides whether a
                // compromised key stays in the published DID document.
                let mut upgrade_needed = None;
                if let Some(ref stored_hmac) = envelope.hmac {
                    verify_envelope_hmac(
                        &integrity_key,
                        "argon2id",
                        salt_b64,
                        &envelope.keys,
                        stored_hmac,
                    )?;
                    if !records_bound {
                        upgrade_needed = Some(UpgradeNeeded::UnboundRecords);
                    }
                } else {
                    upgrade_needed = Some(UpgradeNeeded::MissingIntegrityTag);
                }

                Ok(Self {
                    path: path.as_ref().to_owned(),
                    cipher,
                    integrity_key,
                    salt,
                    records: RwLock::new(envelope.keys),
                    needs_migration: RwLock::new(false),
                    upgrade_needed,
                    records_bound,
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
                crate::os_rng().fill_bytes(&mut salt);

                // Integrity key will be derived properly after migration.
                let integrity_key = derive_integrity_key(passphrase, &salt)?;

                Ok(Self {
                    path: path.as_ref().to_owned(),
                    cipher,
                    integrity_key,
                    salt,
                    records: RwLock::new(records),
                    needs_migration: RwLock::new(true),
                    upgrade_needed: Some(UpgradeNeeded::LegacyKdf),
                    // A legacy store predates binding by definition; migration
                    // re-encrypts every record and sets this.
                    records_bound: false,
                })
            }
        } else {
            // Brand new store — generate a fresh salt.
            let mut salt = [0u8; ARGON2_SALT_LEN];
            crate::os_rng().fill_bytes(&mut salt);
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
                upgrade_needed: None,
                // Nothing to migrate: every record this store will ever hold is
                // written by this build, bound.
                records_bound: true,
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
        let signing_key = SigningKey::generate(&mut crate::os_rng());
        let verifying_key = signing_key.verifying_key();
        let fingerprint = hex::encode(Sha256::digest(verifying_key.as_bytes()));
        let verifying_key_hex = hex::encode(verifying_key.as_bytes());

        let mut nonce_bytes = [0u8; 12];
        crate::os_rng().fill_bytes(&mut nonce_bytes);
        let nonce = <&Nonce<U12>>::from(&nonce_bytes);

        let mut raw = signing_key.to_bytes();
        let encrypted = self
            .cipher
            .encrypt(nonce, Self::record_payload(raw.as_ref(), &fingerprint))
            .map_err(|_| anyhow::anyhow!("AES-GCM encrypt failed"))?;
        raw.zeroize();

        let record = KeyRecord::new(
            encrypted,
            nonce_bytes.to_vec(),
            fingerprint.clone(),
            verifying_key_hex,
        );

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
            algorithm: default_algorithm(),
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

    /// The public key and revocation state of the current key under `key_id`,
    /// without decrypting the private key. Returns `None` if no such key exists.
    ///
    /// `pub` rather than `pub(crate)` because the `did:web` document builder
    /// lives in `dpp-vc` and needs exactly this: public key material and
    /// revocation state, never the private key.
    pub fn public_key(&self, key_id: &str) -> Option<PublicKeyInfo> {
        let map = self.records.read().expect("key store read lock poisoned");
        map.get(key_id).map(PublicKeyInfo::from)
    }

    /// Public keys of all archived records for `key_id`, in the same ascending
    /// timestamp order as [`Self::load_archived_keys`], without decrypting any
    /// private key material.
    pub fn archived_public_keys(&self, key_id: &str) -> Vec<PublicKeyInfo> {
        let prefix = format!("{key_id}#archived-");
        let map = self.records.read().expect("key store read lock poisoned");

        let mut entries: Vec<(&str, &KeyRecord)> = map
            .iter()
            .filter(|(k, _)| k.starts_with(&prefix))
            .map(|(k, v)| (k.as_str(), v))
            .collect();
        entries.sort_by_key(|(k, _)| *k);

        entries
            .into_iter()
            .map(|(_, record)| PublicKeyInfo::from(record))
            .collect()
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

    /// The associated data binding a record's ciphertext to its own identity.
    ///
    /// The **fingerprint**, not the map key. `archive_key` and `rotate_inner`
    /// copy a record to a new map key *without re-encrypting it*, so associated
    /// data derived from the map key would make every archived key undecryptable
    /// the moment it was archived. The fingerprint is the SHA-256 of the public
    /// half: unique per key pair, stored in plaintext beside the ciphertext, and
    /// it travels with the record wherever it is filed.
    ///
    /// What this buys: a record's encrypted private key can no longer be moved
    /// onto a different record's plaintext. Swapping two records' ciphertexts —
    /// or grafting one onto a `verifying_key_hex` and `revoked` flag from
    /// another — now fails to decrypt instead of succeeding quietly.
    fn record_aad(fingerprint: &str) -> &[u8] {
        fingerprint.as_bytes()
    }

    /// An AES-GCM payload carrying `msg` bound to `fingerprint`.
    fn record_payload<'a>(msg: &'a [u8], fingerprint: &'a str) -> Payload<'a, 'a> {
        Payload {
            msg,
            aad: Self::record_aad(fingerprint),
        }
    }

    fn decrypt_record(&self, record: &KeyRecord) -> Result<KeyEntry> {
        let nonce = <&Nonce<U12>>::try_from(record.nonce.as_slice()).map_err(|_| {
            anyhow::anyhow!(
                "stored nonce is not 12 bytes ({} bytes) — corrupt or legacy key record",
                record.nonce.len()
            )
        })?;
        // A pre-V4 record was sealed with no associated data and will not open
        // if we supply any. `open` refuses such a store outright; this path is
        // reached only through `open_and_migrate`, which is re-encrypting them.
        let plaintext = if self.records_bound {
            self.cipher.decrypt(
                nonce,
                Self::record_payload(record.encrypted_signing_key.as_ref(), &record.fingerprint),
            )
        } else {
            self.cipher
                .decrypt(nonce, record.encrypted_signing_key.as_ref())
        };
        let mut raw =
            Zeroizing::new(plaintext.map_err(|_| anyhow::anyhow!("AES-GCM decrypt failed"))?);

        // `Zeroizing` on both: the intermediate array is a second copy of the
        // private key, and the early return below used to drop `raw` without
        // clearing it.
        let bytes: Zeroizing<[u8; 32]> = Zeroizing::new(
            raw.as_slice()
                .try_into()
                .map_err(|_| anyhow::anyhow!("unexpected key length"))?,
        );
        let signing_key = SigningKey::from_bytes(&bytes);
        let verifying_key = signing_key.verifying_key();
        raw.zeroize();

        Ok(KeyEntry {
            fingerprint: record.fingerprint.clone(),
            signing_key,
            verifying_key,
            revoked: record.revoked,
            algorithm: record.algorithm,
        })
    }

    pub(crate) fn persist_envelope(&self, map: &KeyRecordMap) -> Result<()> {
        let keys_clone: KeyRecordMap = map.clone();

        let salt_b64 =
            base64::Engine::encode(&base64::engine::general_purpose::STANDARD, self.salt);
        let hmac_hex =
            compute_envelope_hmac(&self.integrity_key, "argon2id", &salt_b64, &keys_clone)?;

        let envelope = StoreEnvelope {
            version: Some(STORE_VERSION),
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
