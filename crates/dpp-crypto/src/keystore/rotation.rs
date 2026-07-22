use aes_gcm::{
    Nonce,
    aead::{Aead, consts::U12},
};
use anyhow::Result;
use rand::Rng;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::entry::KeyEntry;
use super::store::{KeyRecord, KeyStore};
use ed25519_dalek::SigningKey;

/// Build the archived-record key for `key_id`.
///
/// Suffixed with a time-ordered UUID (v7) rather than a raw nanosecond
/// timestamp: two rotations landing on the same clock tick (or a tight
/// successive-rotation loop, e.g. in tests) previously collided on the same
/// map key, silently overwriting an already-archived record — a signature
/// made with the overwritten key would then never verify again. `load_archived_keys`'s
/// `entries.sort_by_key` still recovers chronological order, since a v7 UUID's
/// leading bits are a millisecond timestamp.
fn archived_key_name(key_id: &str) -> String {
    format!("{key_id}#archived-{}", uuid::Uuid::now_v7())
}

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
            let archived = record.clone();
            let archive_key = archived_key_name(key_id);
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
            .encrypt(nonce, raw.as_ref())
            .map_err(|_| anyhow::anyhow!("AES-GCM encrypt failed"))?;
        raw.zeroize();
        let new_record = KeyRecord::new(
            encrypted,
            nonce_bytes.to_vec(),
            fingerprint.clone(),
            verifying_key_hex,
        );

        {
            let mut map = self.records.write().expect("key store write lock poisoned");
            // Archive the existing current key (if any), marking it revoked when
            // this is a compromise rotation.
            if let Some(record) = map.get(key_id) {
                let archive_name = archived_key_name(key_id);
                let archived = KeyRecord {
                    revoked: revoke_old,
                    ..record.clone()
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
