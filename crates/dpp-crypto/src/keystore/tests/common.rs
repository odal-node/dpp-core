//! Temporary stores the keystore tests open.

use std::path::PathBuf;

use super::super::store::KeyStore;

/// A unique temp-file path for a keystore under test.
pub(super) fn temp_path(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "test-keystore-{label}-{}.json",
        uuid::Uuid::now_v7()
    ))
}

pub(super) fn temp_store() -> KeyStore {
    KeyStore::open(temp_path("default"), "test-passphrase").expect("open key store")
}

/// A fresh [`KeyStore`] plus its backing path, for tests that need to read
/// back or reopen the store file.
pub(super) fn temp_store_at(label: &str, passphrase: &str) -> (KeyStore, PathBuf) {
    let path = temp_path(label);
    let store = KeyStore::open(&path, passphrase).expect("open key store");
    (store, path)
}
