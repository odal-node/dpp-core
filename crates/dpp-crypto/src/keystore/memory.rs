//! [`MemoryKeyStore`], a [`KeyBackend`] that holds its keys in memory.

use std::collections::HashMap;
use std::sync::{PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};

use anyhow::Result;
use zeroize::Zeroizing;

use super::backend::KeyBackend;
use super::entry::{KeyEntry, SigningMaterial, fingerprint_of};
use super::rotation::{ARCHIVE_MARKER, archived_key_name, check_key_id};
use super::store::{PublicKeyInfo, default_algorithm};
use crate::jws::algorithm::KeyAlgorithm;
use crate::jws::public_key::PublicKey;

/// A [`KeyBackend`] that keeps its keys in memory and writes nothing.
///
/// It exists so that the signing, verification, rotation and DID-document tests can run
/// against a second backend, and so that a backend has one short worked example: the
/// file store's encryption, integrity tag, upgrade path and lock are not part of what a
/// backend is, and none of them is here. Keys are gone when it is dropped. It is not for
/// production use.
///
/// Rotation behaves as the file store's does: the replaced key is archived under its own
/// algorithm, a compromise revokes every record that holds the key, and a revoked key
/// stays revoked when it is archived again.
#[derive(Default)]
pub struct MemoryKeyStore {
    records: RwLock<HashMap<String, Record>>,
}

/// One key: its secret, so that [`load_key`](KeyBackend::load_key) can hand out a fresh
/// entry each time, and what is known about it without the secret.
#[derive(Clone)]
struct Record {
    secret: Zeroizing<Vec<u8>>,
    algorithm: KeyAlgorithm,
    public: PublicKey,
    fingerprint: String,
    revoked: bool,
}

impl Record {
    /// A fresh key pair, as a record and as the entry to hand back.
    fn generate(algorithm: KeyAlgorithm) -> (Self, KeyEntry) {
        let material = SigningMaterial::generate(algorithm);
        let public = material.public_key();
        let fingerprint = fingerprint_of(&public);
        let record = Self {
            secret: material.secret(),
            algorithm,
            public,
            fingerprint: fingerprint.clone(),
            revoked: false,
        };
        (record, KeyEntry::new(material, fingerprint, false))
    }

    fn entry(&self) -> Result<KeyEntry> {
        let material = SigningMaterial::from_secret(self.algorithm, &self.secret)?;
        Ok(KeyEntry::new(
            material,
            self.fingerprint.clone(),
            self.revoked,
        ))
    }

    fn info(&self) -> PublicKeyInfo {
        PublicKeyInfo::new(&self.public, self.revoked)
    }
}

impl MemoryKeyStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // A poisoned lock is recovered and not turned into a panic or an error. Each method
    // changes the map only after the one step that can fail, so a thread that panicked
    // while it held the lock left the map as it was, and the next caller can use it.
    fn read(&self) -> RwLockReadGuard<'_, HashMap<String, Record>> {
        self.records.read().unwrap_or_else(PoisonError::into_inner)
    }

    fn write(&self) -> RwLockWriteGuard<'_, HashMap<String, Record>> {
        self.records.write().unwrap_or_else(PoisonError::into_inner)
    }

    /// Poison the lock, as a thread that panics while it holds it does.
    #[cfg(test)]
    pub(super) fn poison_for_test(&self) {
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _held = self.records.write().unwrap_or_else(PoisonError::into_inner);
            panic!("a thread panicked while it held the lock");
        }));
        assert!(poisoned.is_err());
    }
}

impl KeyBackend for MemoryKeyStore {
    fn load_key(&self, key_id: &str) -> Result<KeyEntry> {
        let map = self.read();
        map.get(key_id)
            .ok_or_else(|| anyhow::anyhow!("no key found for {key_id}"))?
            .entry()
    }

    fn public_key(&self, key_id: &str) -> Option<PublicKeyInfo> {
        let map = self.read();
        map.get(key_id).map(Record::info)
    }

    fn archived_public_keys(&self, key_id: &str) -> Vec<PublicKeyInfo> {
        let prefix = format!("{key_id}{ARCHIVE_MARKER}");
        let map = self.read();
        let mut archived: Vec<(&String, &Record)> = map
            .iter()
            .filter(|(name, _)| name.starts_with(&prefix))
            .collect();
        // The names end in a time-ordered UUID, so name order is rotation order.
        archived.sort_by(|a, b| a.0.cmp(b.0));
        archived
            .into_iter()
            .map(|(_, record)| record.info())
            .collect()
    }

    fn list(&self) -> Result<Vec<(String, PublicKeyInfo)>> {
        let map = self.read();
        let mut keys: Vec<(String, PublicKeyInfo)> = map
            .iter()
            .filter(|(name, _)| !name.contains(ARCHIVE_MARKER))
            .map(|(name, record)| (name.clone(), record.info()))
            .collect();
        keys.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(keys)
    }

    fn generate_key_with(&self, key_id: &str, algorithm: KeyAlgorithm) -> Result<KeyEntry> {
        check_key_id(key_id)?;
        let (record, entry) = Record::generate(algorithm);
        let mut map = self.write();
        map.insert(key_id.to_owned(), record);
        Ok(entry)
    }

    fn rotate(
        &self,
        key_id: &str,
        revoke_old: bool,
        algorithm: Option<KeyAlgorithm>,
    ) -> Result<KeyEntry> {
        check_key_id(key_id)?;
        let mut map = self.write();
        let algorithm = algorithm.unwrap_or_else(|| {
            map.get(key_id)
                .map_or_else(default_algorithm, |record| record.algorithm)
        });
        let (record, entry) = Record::generate(algorithm);
        if let Some(current) = map.get(key_id).cloned() {
            if revoke_old {
                // Earlier archives of this key hold the same material, so a compromise
                // revokes them too.
                for stored in map.values_mut() {
                    if stored.fingerprint == current.fingerprint {
                        stored.revoked = true;
                    }
                }
            }
            let archived = Record {
                revoked: revoke_old || current.revoked,
                ..current
            };
            map.insert(archived_key_name(key_id), archived);
        }
        map.insert(key_id.to_owned(), record);
        Ok(entry)
    }
}
