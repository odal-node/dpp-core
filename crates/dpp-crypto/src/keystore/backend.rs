//! [`KeyBackend`], where signing keys live, and the file store as the one that ships.

use std::sync::Arc;

use anyhow::Result;

use super::entry::KeyEntry;
use super::rotation::ARCHIVE_MARKER;
use super::store::{KeyStore, PublicKeyInfo};
use crate::jws::algorithm::KeyAlgorithm;

/// Where signing keys live.
///
/// Everything that signs, verifies or publishes a key takes a `&dyn KeyBackend`, so
/// what it needs from a store is this and nothing more: [`KeyStore`], an encrypted
/// file, is one backend, and a device or a service that signs without a key ever
/// leaving it can be another. A backend names its keys by an id, and under each id it
/// keeps a **current** key and the **archived** predecessors a rotation left behind.
///
/// # What a caller gets
///
/// [`load_key`](Self::load_key) returns a [`KeyEntry`], which signs and reports its own
/// public key and algorithm, and which is one key for as long as it lives. A caller
/// that builds a token from the entry, so that the header's `alg` and `kid` come from
/// the key that then signs, cannot be handed a different key by a rotation in
/// between. Nothing in this trait returns private key material, so a backend whose keys
/// cannot be read out has nothing to hide behind a method that cannot answer.
///
/// No key or signature size is assumed here. The algorithm comes from the key.
///
/// # Rotation belongs to the backend
///
/// [`rotate`](Self::rotate) archives the current key and installs its successor, and
/// when the old key is compromised it revokes **every** copy of it, in one step. That is
/// the backend's to do, not something to compose above it from smaller calls: the file
/// store writes both changes in a single atomic file replacement, so a crash leaves the
/// old state or the new one and never an archive with no successor, and a revoked key is
/// never left valid in an older archive. A backend that cannot make that step atomic
/// should say so in its own documentation.
///
/// # What stays outside
///
/// How a backend is opened or reached is its own: [`KeyStore::open`] takes a path and a
/// passphrase, and nothing here does. The file store's other concerns, such as the
/// integrity tag over its file, the upgrade of an older file, and decrypting an
/// archived key, are not part of the contract.
pub trait KeyBackend: Send + Sync {
    /// The current key under `key_id`.
    ///
    /// The entry carries the revocation state the backend recorded when it was loaded:
    /// an entry loaded before a revocation keeps its copy and still signs, so load
    /// the key again after revoking it.
    ///
    /// # Errors
    ///
    /// When no key is recorded under `key_id`, or the backend cannot read it.
    fn load_key(&self, key_id: &str) -> Result<KeyEntry>;

    /// The public key and revocation state of the current key under `key_id`, or `None`
    /// if there is none.
    fn public_key(&self, key_id: &str) -> Option<PublicKeyInfo>;

    /// The public keys of the archived predecessors of `key_id`, oldest first.
    ///
    /// Public halves only. A signature made before a rotation verifies against one of
    /// these, which is why a published key set lists them, and a revoked one is marked
    /// so that it can be left out.
    fn archived_public_keys(&self, key_id: &str) -> Vec<PublicKeyInfo>;

    /// Every current key, with its id, sorted by id. Archived keys are not listed: they
    /// are reached through [`archived_public_keys`](Self::archived_public_keys).
    ///
    /// # Errors
    ///
    /// When the backend cannot be read.
    fn list(&self) -> Result<Vec<(String, PublicKeyInfo)>>;

    /// Generate a key of `algorithm` under `key_id`, replacing any key there, and return it.
    ///
    /// Replacing is not rotating: the old key is not archived, so a signature it made no
    /// longer has a key to verify against. Use [`rotate_key`](Self::rotate_key) to keep it.
    ///
    /// # Errors
    ///
    /// When the backend cannot generate or record the key.
    fn generate_key_with(&self, key_id: &str, algorithm: KeyAlgorithm) -> Result<KeyEntry>;

    /// Archive the current key under `key_id` and install a fresh one, in one step, and
    /// return the new key.
    ///
    /// `algorithm` is the new key's, and the current key's own when it is `None`, or
    /// Ed25519 when `key_id` has no key yet. The archived key keeps its own algorithm, so
    /// a published key set after a cross-algorithm rotation lists both, each in its own
    /// shape.
    ///
    /// With `revoke_old`, the archived key is marked revoked, and so is every other
    /// record that holds the same key material: a compromise must not leave the key
    /// valid in an older archive. Without it, a current key that is already revoked is
    /// archived revoked, because a rotation never makes a revoked key valid again.
    ///
    /// Callers use the three named forms below.
    ///
    /// # Errors
    ///
    /// When the backend cannot generate or record the new key. The old state is kept.
    fn rotate(
        &self,
        key_id: &str,
        revoke_old: bool,
        algorithm: Option<KeyAlgorithm>,
    ) -> Result<KeyEntry>;

    /// Whether a current key is recorded under `key_id`.
    fn has_key(&self, key_id: &str) -> bool {
        self.public_key(key_id).is_some()
    }

    /// Generate an Ed25519 key under `key_id`. See [`generate_key_with`](Self::generate_key_with).
    ///
    /// # Errors
    ///
    /// As [`generate_key_with`](Self::generate_key_with).
    fn generate_key(&self, key_id: &str) -> Result<KeyEntry> {
        self.generate_key_with(key_id, KeyAlgorithm::Ed25519)
    }

    /// Routine rotation: archive the current key, kept valid so older signatures still
    /// verify, and install a fresh one of the same algorithm.
    ///
    /// # Errors
    ///
    /// As [`rotate`](Self::rotate).
    fn rotate_key(&self, key_id: &str) -> Result<KeyEntry> {
        self.rotate(key_id, false, None)
    }

    /// [`rotate_key`](Self::rotate_key), installing a key of `algorithm`.
    ///
    /// # Errors
    ///
    /// As [`rotate`](Self::rotate).
    fn rotate_key_to(&self, key_id: &str, algorithm: KeyAlgorithm) -> Result<KeyEntry> {
        self.rotate(key_id, false, Some(algorithm))
    }

    /// Rotation on **compromise**: archive the current key as revoked, so every key set
    /// built from the backend drops it and its signatures no longer verify, and install a
    /// fresh one of the same algorithm.
    ///
    /// # Errors
    ///
    /// As [`rotate`](Self::rotate).
    fn revoke_and_rotate(&self, key_id: &str) -> Result<KeyEntry> {
        self.rotate(key_id, true, None)
    }
}

/// A shared backend is a backend, so a host that keeps its store in an `Arc` passes
/// `&Arc<KeyStore>` wherever a `&dyn KeyBackend` is taken.
impl<T: KeyBackend + ?Sized> KeyBackend for Arc<T> {
    fn load_key(&self, key_id: &str) -> Result<KeyEntry> {
        (**self).load_key(key_id)
    }

    fn public_key(&self, key_id: &str) -> Option<PublicKeyInfo> {
        (**self).public_key(key_id)
    }

    fn archived_public_keys(&self, key_id: &str) -> Vec<PublicKeyInfo> {
        (**self).archived_public_keys(key_id)
    }

    fn list(&self) -> Result<Vec<(String, PublicKeyInfo)>> {
        (**self).list()
    }

    fn generate_key_with(&self, key_id: &str, algorithm: KeyAlgorithm) -> Result<KeyEntry> {
        (**self).generate_key_with(key_id, algorithm)
    }

    fn rotate(
        &self,
        key_id: &str,
        revoke_old: bool,
        algorithm: Option<KeyAlgorithm>,
    ) -> Result<KeyEntry> {
        (**self).rotate(key_id, revoke_old, algorithm)
    }

    fn has_key(&self, key_id: &str) -> bool {
        (**self).has_key(key_id)
    }
}

/// The encrypted file store, as a [`KeyBackend`]. Each method is the inherent one of the
/// same name, which keeps its documentation of what the file does.
impl KeyBackend for KeyStore {
    fn load_key(&self, key_id: &str) -> Result<KeyEntry> {
        Self::load_key(self, key_id)
    }

    fn public_key(&self, key_id: &str) -> Option<PublicKeyInfo> {
        Self::public_key(self, key_id)
    }

    fn archived_public_keys(&self, key_id: &str) -> Vec<PublicKeyInfo> {
        Self::archived_public_keys(self, key_id)
    }

    fn list(&self) -> Result<Vec<(String, PublicKeyInfo)>> {
        let map = self
            .records
            .read()
            .map_err(|_| anyhow::anyhow!("key store lock poisoned"))?;
        let mut keys: Vec<(String, PublicKeyInfo)> = map
            .iter()
            .filter(|(key_id, _)| !key_id.contains(ARCHIVE_MARKER))
            .map(|(key_id, record)| (key_id.clone(), PublicKeyInfo::from(record)))
            .collect();
        keys.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(keys)
    }

    fn generate_key_with(&self, key_id: &str, algorithm: KeyAlgorithm) -> Result<KeyEntry> {
        Self::generate_key_with(self, key_id, algorithm)
    }

    fn rotate(
        &self,
        key_id: &str,
        revoke_old: bool,
        algorithm: Option<KeyAlgorithm>,
    ) -> Result<KeyEntry> {
        self.rotate_inner(key_id, revoke_old, algorithm)
    }

    fn has_key(&self, key_id: &str) -> bool {
        Self::has_key(self, key_id)
    }
}
