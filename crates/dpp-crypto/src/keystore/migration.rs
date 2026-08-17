use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, consts::U12},
};
use anyhow::{Context, Result};
use rand::Rng;
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
        // The permissive door: this is the one function allowed to open a store
        // that predates a current security property, because it is the one that
        // repairs it. `open` refuses them and points here.
        let store = Self::open_permissively(path.as_ref(), passphrase)?;
        if store.migrate_if_needed(passphrase)? {
            // Re-open strictly. The upgraded file must satisfy `open` on its own
            // terms — if it does not, the migration did not finish the job and
            // saying so beats handing back a store nobody checked.
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
        // Three things can require an upgrade and they are not independent: a
        // legacy-KDF store also lacks an integrity tag and record binding. One
        // pass re-encrypts every record under the current key with its
        // fingerprint as associated data and rewrites the envelope, which
        // satisfies all three at once.
        let Some(reason) = self.upgrade_needed else {
            return Ok(false);
        };

        tracing::info!(?reason, "upgrading key store to the current format");

        // Decrypt all records with the old cipher, re-encrypt with the new one.
        let new_key = derive_aes_key_argon2(passphrase, &self.salt)?;
        let new_cipher = Aes256Gcm::new(&new_key);

        let mut map = self.records.write().expect("key store write lock");
        let mut migrated = KeyRecordMap::with_capacity(map.len());

        for (id, record) in map.iter() {
            // Decrypt with legacy cipher.
            let nonce = <&Nonce<U12>>::try_from(record.nonce.as_slice()).map_err(|_| {
                anyhow::anyhow!(
                    "stored nonce is not 12 bytes ({} bytes) for key {id} — corrupt or legacy record",
                    record.nonce.len()
                )
            })?;
            // Read with whatever binding the *stored* record has, write with the
            // current one.
            let opened = if self.records_bound {
                self.cipher.decrypt(
                    nonce,
                    aes_gcm::aead::Payload {
                        msg: record.encrypted_signing_key.as_ref(),
                        aad: record.fingerprint.as_bytes(),
                    },
                )
            } else {
                self.cipher
                    .decrypt(nonce, record.encrypted_signing_key.as_ref())
            };
            let mut raw = opened.map_err(|_| {
                anyhow::anyhow!("AES-GCM decrypt failed during migration for key {id}")
            })?;

            // Re-encrypt with new cipher + fresh nonce.
            let mut nonce_bytes = [0u8; 12];
            crate::os_rng().fill_bytes(&mut nonce_bytes);
            let new_nonce = <&Nonce<U12>>::from(&nonce_bytes);
            let encrypted = new_cipher
                .encrypt(
                    new_nonce,
                    aes_gcm::aead::Payload {
                        msg: raw.as_ref(),
                        aad: record.fingerprint.as_bytes(),
                    },
                )
                .map_err(|_| anyhow::anyhow!("AES-GCM encrypt failed during migration"))?;
            raw.zeroize();

            migrated.insert(
                id.clone(),
                KeyRecord {
                    encrypted_signing_key: encrypted,
                    nonce: nonce_bytes.to_vec(),
                    ..record.clone()
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
        tracing::info!("key store upgraded to the current format");
        Ok(true)
    }
}
