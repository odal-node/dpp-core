//! Write-counting key backend for read-only path tests.

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::{KeyBackend, KeyEntry, MemoryKeyStore, PublicKeyInfo};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Default)]
pub(super) struct CountingKeyBackend {
    store: MemoryKeyStore,
    pub(super) writes: AtomicUsize,
}

impl KeyBackend for CountingKeyBackend {
    fn load_key(&self, key_id: &str) -> anyhow::Result<KeyEntry> {
        self.store.load_key(key_id)
    }

    fn public_key(&self, key_id: &str) -> Option<PublicKeyInfo> {
        self.store.public_key(key_id)
    }

    fn archived_public_keys(&self, key_id: &str) -> Vec<PublicKeyInfo> {
        self.store.archived_public_keys(key_id)
    }

    fn list(&self) -> anyhow::Result<Vec<(String, PublicKeyInfo)>> {
        self.store.list()
    }

    fn generate_key_with(&self, key_id: &str, algorithm: KeyAlgorithm) -> anyhow::Result<KeyEntry> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        self.store.generate_key_with(key_id, algorithm)
    }

    fn rotate(
        &self,
        key_id: &str,
        revoke_old: bool,
        algorithm: Option<KeyAlgorithm>,
    ) -> anyhow::Result<KeyEntry> {
        self.writes.fetch_add(1, Ordering::SeqCst);
        self.store.rotate(key_id, revoke_old, algorithm)
    }
}
