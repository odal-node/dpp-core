//! Shared `#[cfg(test)]`-only fixtures, used across module test suites.

use crate::keystore::KeyStore;

/// A unique path under the temp dir, named for `label` with a v7 UUID.
pub(crate) fn temp_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("test-{label}-{}.json", uuid::Uuid::now_v7()))
}

/// A fresh [`KeyStore`] at a unique temp path, with one key already generated.
pub(crate) fn temp_store(label: &str, key_id: &str) -> KeyStore {
    let store = KeyStore::open(temp_path(label), "test-pass").expect("open store");
    store.generate_key(key_id).expect("generate key");
    store
}
