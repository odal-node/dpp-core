//! A store written to disk and read back.

use std::collections::HashMap;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, consts::U12},
};
use ed25519_dalek::SigningKey;
use rand::Rng;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::super::crypto::derive_aes_key_sha256;
use super::super::store::{KeyRecord, KeyRecordMap, KeyStore, default_algorithm};
use super::common::*;

#[test]
fn generate_then_load_roundtrip() {
    let store = temp_store();
    let generated = store.generate_key("issuer-1").expect("generate key");
    let loaded = store.load_key("issuer-1").expect("load key");
    assert_eq!(generated.fingerprint, loaded.fingerprint);
    assert_eq!(generated.public_key(), loaded.public_key());
}

#[test]
fn stored_bytes_differ_from_plaintext() {
    let (store, path) = temp_store_at("enc", "test-passphrase");
    let key = store.generate_key("issuer-enc").expect("generate");

    let raw_file = std::fs::read_to_string(&path).expect("read file");
    let plaintext_hex = hex::encode(key.secret_for_test().as_slice());
    assert!(
        !raw_file.contains(&plaintext_hex),
        "plaintext key bytes must not appear in the store file"
    );
}

#[test]
fn store_file_contains_argon2id_kdf_marker() {
    let (store, path) = temp_store_at("kdf", "test-passphrase");
    store.generate_key("issuer-kdf").expect("generate");

    let raw_file = std::fs::read_to_string(&path).expect("read file");
    assert!(
        raw_file.contains("argon2id"),
        "store file must contain argon2id KDF marker"
    );
    assert!(
        raw_file.contains("salt"),
        "store file must contain salt field"
    );
}

#[test]
fn reopen_store_from_disk_with_argon2id() {
    let (store, path) = temp_store_at("reopen", "my-passphrase");
    let generated = store.generate_key("issuer-reopen").expect("generate");
    drop(store);

    // Re-open from disk.
    let store2 = KeyStore::open(&path, "my-passphrase").expect("reopen");
    let loaded = store2.load_key("issuer-reopen").expect("load");
    assert_eq!(generated.fingerprint, loaded.fingerprint);
}

#[test]
fn legacy_sha256_store_can_be_opened_and_migrated() {
    // Simulate a V1 (raw HashMap) key store created with SHA-256.
    let path = temp_path("legacy");

    // Create a legacy-format store by directly writing the old format.
    let passphrase = "legacy-pass";
    let legacy_key = derive_aes_key_sha256(passphrase);
    let cipher = Aes256Gcm::new(&legacy_key);

    let signing_key = SigningKey::generate(&mut crate::os_rng());
    let verifying_key = signing_key.verifying_key();
    let fingerprint = hex::encode(Sha256::digest(verifying_key.as_bytes()));

    let mut nonce_bytes = [0u8; 12];
    crate::os_rng().fill_bytes(&mut nonce_bytes);
    let nonce = <&Nonce<U12>>::from(&nonce_bytes);
    let mut raw = signing_key.to_bytes();
    let encrypted = cipher.encrypt(nonce, raw.as_ref()).expect("encrypt");
    raw.zeroize();

    let record = KeyRecord {
        encrypted_signing_key: encrypted,
        nonce: nonce_bytes.to_vec(),
        fingerprint: fingerprint.clone(),
        verifying_key_hex: hex::encode(verifying_key.as_bytes()),
        revoked: false,
        algorithm: default_algorithm(),
    };
    let mut map = HashMap::new();
    map.insert("legacy-key".to_string(), record);
    let bytes = serde_json::to_vec(&map).expect("serialize");
    std::fs::write(&path, bytes).expect("write");

    // `open` refuses it, by name, and says where to go.
    // `expect_err` would need `KeyStore: Debug`, and it deliberately has none —
    // the struct holds a passphrase-derived cipher.
    let Err(refused) = KeyStore::open(&path, passphrase) else {
        panic!("a legacy-KDF store must not open through `open`");
    };
    let message = refused.to_string();
    assert!(message.contains("legacy SHA-256 KDF"), "{message}");
    assert!(message.contains("open_and_migrate"), "{message}");

    // The migration door opens it, upgrades it, and hands back a store that
    // satisfies `open` on its own terms.
    let store = KeyStore::open_and_migrate(&path, passphrase).expect("migrate legacy store");
    let loaded = store.load_key("legacy-key").expect("load legacy key");
    assert_eq!(loaded.fingerprint, fingerprint);

    let raw_file = std::fs::read_to_string(&path).expect("read");
    assert!(raw_file.contains("argon2id"), "file must be migrated");

    // The lock was held through the upgrade and the re-open, and the handle that came
    // back still holds it: a second handle is refused.
    let Err(second) = KeyStore::open(&path, passphrase) else {
        panic!("a second handle opened a store that a migrated handle holds");
    };
    assert!(
        second
            .downcast_ref::<super::super::KeyStoreLocked>()
            .is_some(),
        "{second:#}"
    );
    drop(store);

    // And it is now openable strictly — which is the property that says the
    // upgrade actually finished rather than merely ran.
    KeyStore::open(&path, passphrase).expect("migrated store opens strictly");
}

#[test]
fn malformed_nonce_returns_error_not_panic() {
    // A legacy V1 store (raw HashMap, no HMAC) whose record carries a truncated
    // nonce. Opening must succeed; loading must return an error rather than
    // panicking on the 12-byte nonce check.
    let path = temp_path("badnonce");

    let record = KeyRecord {
        encrypted_signing_key: vec![0u8; 48],
        nonce: vec![0u8; 11], // truncated — not 12 bytes
        fingerprint: "deadbeef".to_string(),
        verifying_key_hex: format!("{:064x}", 0),
        revoked: false,
        algorithm: default_algorithm(),
    };
    let mut map: KeyRecordMap = HashMap::new();
    map.insert("bad-key".to_string(), record);
    std::fs::write(&path, serde_json::to_vec(&map).expect("serialize")).expect("write");

    // Reached through the migration door, since a raw-map store is legacy by
    // definition. Migration itself fails on the malformed record, so the check
    // is on the permissive open that precedes it.
    let store = KeyStore::open_permissively(&path, "any-pass").expect("open legacy store");
    assert!(
        store.load_key("bad-key").is_err(),
        "malformed nonce must return Err, not panic"
    );
}
