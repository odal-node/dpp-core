//! Shared `#[cfg(test)]`-only fixtures, used across module test suites.

use crate::keystore::KeyStore;

/// A fresh [`KeyStore`] at a unique temp path, with one key already generated.
pub(crate) fn temp_store(label: &str, key_id: &str) -> KeyStore {
    let path = std::env::temp_dir().join(format!("test-{label}-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(path, "test-pass").expect("open store");
    store.generate_key(key_id).expect("generate key");
    store
}
