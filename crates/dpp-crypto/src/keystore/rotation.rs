use aes_gcm::{
    Nonce,
    aead::{Aead, OsRng},
};
use anyhow::Result;
use rand::RngCore;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::{KeyRecord, KeyStore, default_algorithm};
use ed25519_dalek::SigningKey;

use crate::keystore::KeyEntry;

impl KeyStore {
    /// Archive the current key under a timestamped key so it can still be used
    /// to verify older signatures after rotation.
    pub fn archive_key(&self, key_id: &str) -> Result<()> {
        if *self.needs_migration.read().expect("lock") {
            anyhow::bail!(
                "key store requires KDF migration before writes are allowed — \
                 call migrate_if_needed() first"
            );
        }
        let mut map = self.records.write().expect("key store write lock poisoned");
        if let Some(record) = map.get(key_id) {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos();
            let archive_key = format!("{key_id}#archived-{ts}");
            let archived = KeyRecord {
                encrypted_signing_key: record.encrypted_signing_key.clone(),
                nonce: record.nonce.clone(),
                fingerprint: record.fingerprint.clone(),
                verifying_key_hex: record.verifying_key_hex.clone(),
                revoked: record.revoked,
                algorithm: record.algorithm.clone(),
            };
            map.insert(archive_key, archived);
            self.persist_envelope(&map)?;
        }
        Ok(())
    }

    /// Atomically rotate the key: archive the current key (kept valid so older
    /// signatures still verify) and install a fresh current key, in a **single**
    /// persisted write. Use for routine/hygiene rotation. Returns the new key.
    ///
    /// Unlike calling [`archive_key`](Self::archive_key) then
    /// [`generate_key`](Self::generate_key), there is no intermediate on-disk
    /// state where the archive exists but the new key does not (identity I4).
    pub fn rotate_key(&self, key_id: &str) -> Result<KeyEntry> {
        self.rotate_inner(key_id, false)
    }

    /// Atomically **revoke** the current key and install a fresh one. The old key
    /// is archived but marked revoked, so [`crate::identity::did_builder`] drops it from
    /// the published DID document and signatures it produced no longer verify.
    /// Use this on key **compromise** (vs. hygiene rotation, where the old key
    /// stays valid — see [`rotate_key`](Self::rotate_key)).
    pub fn revoke_and_rotate(&self, key_id: &str) -> Result<KeyEntry> {
        self.rotate_inner(key_id, true)
    }

    fn rotate_inner(&self, key_id: &str, revoke_old: bool) -> Result<KeyEntry> {
        if *self.needs_migration.read().expect("lock") {
            anyhow::bail!(
                "key store requires KDF migration before writes are allowed — \
                 call migrate_if_needed() first"
            );
        }

        // Prepare the new key material up front so the lock-held section is just
        // the in-memory swap + single persist.
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
        let new_record = KeyRecord {
            encrypted_signing_key: encrypted,
            nonce: nonce_bytes.to_vec(),
            fingerprint: fingerprint.clone(),
            verifying_key_hex,
            revoked: false,
            algorithm: default_algorithm(),
        };

        {
            let mut map = self.records.write().expect("key store write lock poisoned");
            // Archive the existing current key (if any), marking it revoked when
            // this is a compromise rotation.
            if let Some(record) = map.get(key_id) {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                let archive_name = format!("{key_id}#archived-{ts}");
                let archived = KeyRecord {
                    encrypted_signing_key: record.encrypted_signing_key.clone(),
                    nonce: record.nonce.clone(),
                    fingerprint: record.fingerprint.clone(),
                    verifying_key_hex: record.verifying_key_hex.clone(),
                    revoked: revoke_old,
                    algorithm: record.algorithm.clone(),
                };
                map.insert(archive_name, archived);
            }
            map.insert(key_id.to_owned(), new_record);
            self.persist_envelope(&map)?;
        }

        Ok(KeyEntry {
            signing_key,
            verifying_key,
            fingerprint,
            revoked: false,
        })
    }
}
