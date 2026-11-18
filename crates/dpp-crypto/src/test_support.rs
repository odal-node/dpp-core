//! Shared `#[cfg(test)]`-only fixtures, used across module test suites.

use crate::keystore::KeyStore;

/// The name every test path under the temp dir shares: `test-<label>-<v7 UUID>`.
fn unique_name(label: &str) -> String {
    format!("test-{label}-{}", uuid::Uuid::now_v7())
}

/// A unique path under the temp dir, named for `label` with a v7 UUID; a `.json` file.
pub(crate) fn temp_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(format!("{}.json", unique_name(label)))
}

/// A unique directory path under the temp dir, named for `label` with a v7 UUID. The
/// directory is not created.
pub(crate) fn temp_dir(label: &str) -> std::path::PathBuf {
    std::env::temp_dir().join(unique_name(label))
}

/// An empty [`KeyStore`] at `path`. Opening writes nothing: the file appears on the
/// first save.
pub(crate) fn open_empty_store(path: &std::path::Path) -> KeyStore {
    KeyStore::open(path, "test-pass").expect("open store")
}

/// A fresh [`KeyStore`] at a unique temp path, with one key already generated.
pub(crate) fn temp_store(label: &str, key_id: &str) -> KeyStore {
    let store = open_empty_store(&temp_path(label));
    store.generate_key(key_id).expect("generate key");
    store
}
