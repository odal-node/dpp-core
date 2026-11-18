//! Shared `#[cfg(test)]`-only fixtures for this crate's test suites.

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyStore;

/// Every algorithm a key can be generated under, in the order a test loops over.
pub(crate) const EVERY_ALGORITHM: [KeyAlgorithm; 4] = [
    KeyAlgorithm::Ed25519,
    KeyAlgorithm::EcdsaP256,
    KeyAlgorithm::EcdsaP384,
    KeyAlgorithm::EcdsaP521,
];

/// A fresh [`KeyStore`] at a unique temp path, with one key already generated.
///
/// Duplicated from `dpp-crypto`'s own test support rather than shared: a
/// `#[cfg(test)]` module is private to its crate, and exposing it across a
/// crate boundary would mean shipping test scaffolding in the published API.
pub(crate) fn temp_store(label: &str, key_id: &str) -> KeyStore {
    temp_store_with(label, key_id, KeyAlgorithm::Ed25519)
}

/// [`temp_store`], with a key of `algorithm` in place of an Ed25519 one.
pub(crate) fn temp_store_with(label: &str, key_id: &str, algorithm: KeyAlgorithm) -> KeyStore {
    let path = std::env::temp_dir().join(format!("test-{label}-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(path, "test-pass").expect("open store");
    store
        .generate_key_with(key_id, algorithm)
        .expect("generate key");
    store
}
