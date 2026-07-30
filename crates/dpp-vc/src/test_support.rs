//! Shared `#[cfg(test)]`-only fixtures for this crate's test suites.

use dpp_crypto::keystore::KeyStore;

/// A fresh [`KeyStore`] at a unique temp path, with one key already generated.
///
/// Duplicated from `dpp-crypto`'s own test support rather than shared: a
/// `#[cfg(test)]` module is private to its crate, and exposing it across a
/// crate boundary would mean shipping test scaffolding in the published API.
pub(crate) fn temp_store(label: &str, key_id: &str) -> KeyStore {
    let path = std::env::temp_dir().join(format!("test-{label}-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(path, "test-pass").expect("open store");
    store.generate_key(key_id).expect("generate key");
    store
}
