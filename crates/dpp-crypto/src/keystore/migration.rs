use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng},
};
use anyhow::{Context, Result};
use rand::RngCore;
use zeroize::Zeroize;

use super::crypto::derive_aes_key_argon2;
use super::store::{KeyRecord, KeyRecordMap, KeyStore};

impl KeyStore {
    /// Open the key store, run `migrate_if_needed` if the store uses the legacy
    /// SHA-256 KDF, and — if migration actually ran — re-open the file so
    /// `self.cipher` reflects the new Argon2id key.
    ///
    /// This is the recommended entry point for production code. For stores
    /// already at V2/V3 (Argon2id) it is identical to a single `open` call.
    pub fn open_and_migrate(path: impl AsRef<std::path::Path>, passphrase: &str) -> Result<Self> {
        let store = Self::open(path.as_ref(), passphrase)?;
        if store.migrate_if_needed(passphrase)? {
            // Re-open with the migrated file so the in-memory cipher is updated.
            Self::open(path, passphrase)
        } else {
            Ok(store)
        }
    }

    /// If this store was opened from a legacy format, re-encrypt all keys
    /// with the Argon2id-derived key and persist. Call this once after
    /// opening and verifying the passphrase works (e.g. by loading a key).
    ///
    /// Returns `true` if migration ran, `false` if the store was already at
    /// V2/V3. Use `open_and_migrate` in production to avoid the post-migration
    /// cipher inconsistency (this object's `self.cipher` is not updated here).
    pub fn migrate_if_needed(&self, passphrase: &str) -> Result<bool> {
        let needs = *self.needs_migration.read().expect("lock");
        if !needs {
            return Ok(false);
        }

        tracing::info!("migrating key store from SHA-256 to Argon2id KDF");

        // Decrypt all records with the old cipher, re-encrypt with the new one.
        let new_key = derive_aes_key_argon2(passphrase, &self.salt)?;
        let new_cipher = Aes256Gcm::new(&new_key);

        let mut map = self.records.write().expect("key store write lock");
        let mut migrated = KeyRecordMap::with_capacity(map.len());

        for (id, record) in map.iter() {
            // Decrypt with legacy cipher.
            let nonce = Nonce::from_slice(&record.nonce);
            let mut raw = self
                .cipher
                .decrypt(nonce, record.encrypted_signing_key.as_ref())
                .map_err(|_| {
                    anyhow::anyhow!("AES-GCM decrypt failed during migration for key {id}")
                })?;

            // Re-encrypt with new cipher + fresh nonce.
            let mut nonce_bytes = [0u8; 12];
            OsRng.fill_bytes(&mut nonce_bytes);
            let new_nonce = Nonce::from_slice(&nonce_bytes);
            let encrypted = new_cipher
                .encrypt(new_nonce, raw.as_ref())
                .map_err(|_| anyhow::anyhow!("AES-GCM encrypt failed during migration"))?;
            raw.zeroize();

            migrated.insert(
                id.clone(),
                KeyRecord {
                    encrypted_signing_key: encrypted,
                    nonce: nonce_bytes.to_vec(),
                    fingerprint: record.fingerprint.clone(),
                    verifying_key_hex: record.verifying_key_hex.clone(),
                    revoked: record.revoked,
                    algorithm: record.algorithm.clone(),
                },
            );
        }

        *map = migrated;
        self.persist_envelope(&map)
            .context("Failed to persist migrated key store")?;

        drop(map);
        *self.needs_migration.write().expect("lock") = false;

        // self.cipher still holds the old key; callers must use open_and_migrate
        // (which re-opens the file) rather than continuing to use this object.
        tracing::info!("key store migrated from SHA-256 to Argon2id");
        Ok(true)
    }
}
