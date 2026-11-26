//! Migration of older store files, and what binds a record to its identity.

use std::collections::HashMap;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, consts::U12},
};
use ed25519_dalek::SigningKey;
use rand::Rng;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use std::path::PathBuf;

use super::super::crypto::compute_envelope_hmac;
use super::super::store::{KeyRecord, KeyRecordMap, KeyStore, default_algorithm};
use super::common::*;

/// Strip a field from a store file, producing an older shape on disk.
fn strip_field(path: &PathBuf, field: &str) {
    let raw = std::fs::read_to_string(path).expect("read");
    let mut envelope: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    envelope
        .as_object_mut()
        .expect("envelope is an object")
        .remove(field);
    std::fs::write(path, serde_json::to_vec(&envelope).expect("serialize")).expect("write");
}

/// A current-format store with its integrity tag removed is refused by both doors.
///
/// Without the tag, every plaintext field is unauthenticated — including
/// `revoked`, which `dpp-vc`'s `did:web` builder reads to drop a compromised key
/// from the published DID document. Every store this code writes carries a tag,
/// so a current-format store without one has been tampered with, and the
/// migration door must not be a way to authenticate what was changed.
#[test]
fn a_current_store_with_no_integrity_tag_is_refused_by_both_doors() {
    let (store, path) = temp_store_at("nohmac", "pw");
    store.generate_key("iss").expect("generate");
    drop(store);

    strip_field(&path, "hmac");

    let Err(refused) = KeyStore::open(&path, "pw") else {
        panic!("a store with no integrity tag must not open");
    };
    assert!(refused.to_string().contains("integrity tag"), "{refused}");

    let Err(refused) = KeyStore::open_and_migrate(&path, "pw") else {
        panic!("migration must not authenticate a store with no integrity tag");
    };
    assert!(refused.to_string().contains("integrity tag"), "{refused}");
}

/// A migrated record's public key must be the one its private key derives.
///
/// The public key and fingerprint sit beside the encrypted private key in
/// plaintext. Migration re-authenticates whatever it carries, so it checks them
/// against the decrypted key first and refuses a store where they disagree.
#[test]
fn migration_refuses_a_public_key_that_does_not_match_its_private_key() {
    // An unbound store with a valid tag over records that carry another key's
    // public half: migration reads it, and must refuse it before re-tagging.
    let path = temp_path("mismatch");
    let passphrase = "pw";
    let mut salt = [0u8; 16];
    crate::os_rng().fill_bytes(&mut salt);
    let cipher = Aes256Gcm::new(
        &super::super::crypto::derive_aes_key_argon2(passphrase, &salt).expect("derive aes key"),
    );

    let signing_key = SigningKey::generate(&mut crate::os_rng());
    let other = SigningKey::generate(&mut crate::os_rng()).verifying_key();
    let mut nonce_bytes = [0u8; 12];
    crate::os_rng().fill_bytes(&mut nonce_bytes);
    let nonce = <&Nonce<U12>>::from(&nonce_bytes);
    let mut raw = signing_key.to_bytes();
    let encrypted = cipher.encrypt(nonce, raw.as_ref()).expect("encrypt");
    raw.zeroize();

    let mut keys: KeyRecordMap = HashMap::new();
    keys.insert(
        "iss".to_owned(),
        KeyRecord {
            encrypted_signing_key: encrypted,
            nonce: nonce_bytes.to_vec(),
            fingerprint: hex::encode(Sha256::digest(other.as_bytes())),
            verifying_key_hex: hex::encode(other.as_bytes()),
            revoked: false,
            algorithm: default_algorithm(),
        },
    );

    let salt_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt);
    let integrity_key =
        super::super::crypto::derive_integrity_key(passphrase, &salt).expect("integrity key");
    let hmac = compute_envelope_hmac(&integrity_key, "argon2id", &salt_b64, &keys).expect("hmac");
    let envelope = serde_json::json!({
        "kdf": "argon2id",
        "salt": salt_b64,
        "hmac": hmac,
        "keys": keys,
    });
    std::fs::write(&path, serde_json::to_vec(&envelope).expect("serialize")).expect("write");

    let Err(refused) = KeyStore::open_and_migrate(&path, passphrase) else {
        panic!("a mismatched public key must not be migrated");
    };
    assert!(refused.to_string().contains("does not match"), "{refused}");
}

/// A genuine pre-binding (V3) store is refused, then upgraded.
///
/// Built by hand rather than by stripping a marker off a current store: a V3
/// store's records were sealed with **no** associated data, and a file that
/// merely claims to be V3 while holding bound records is not the thing under
/// test. The migration has to actually re-encrypt, so it has to actually decrypt
/// the old shape first.
#[test]
fn a_store_with_unbound_records_is_refused_then_upgraded() {
    let path = temp_path("unbound");
    let passphrase = "pw";

    let mut salt = [0u8; 16];
    crate::os_rng().fill_bytes(&mut salt);
    let cipher_key =
        super::super::crypto::derive_aes_key_argon2(passphrase, &salt).expect("derive aes key");
    let cipher = Aes256Gcm::new(&cipher_key);

    let signing_key = SigningKey::generate(&mut crate::os_rng());
    let verifying_key = signing_key.verifying_key();
    let fingerprint = hex::encode(Sha256::digest(verifying_key.as_bytes()));

    let mut nonce_bytes = [0u8; 12];
    crate::os_rng().fill_bytes(&mut nonce_bytes);
    let nonce = <&Nonce<U12>>::from(&nonce_bytes);
    let mut raw = signing_key.to_bytes();
    // No associated data — this is what V3 wrote.
    let encrypted = cipher.encrypt(nonce, raw.as_ref()).expect("encrypt");
    raw.zeroize();

    let mut keys: KeyRecordMap = HashMap::new();
    keys.insert(
        "iss".to_owned(),
        KeyRecord {
            encrypted_signing_key: encrypted,
            nonce: nonce_bytes.to_vec(),
            fingerprint: fingerprint.clone(),
            verifying_key_hex: hex::encode(verifying_key.as_bytes()),
            revoked: false,
            algorithm: default_algorithm(),
        },
    );

    let salt_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, salt);
    let integrity_key =
        super::super::crypto::derive_integrity_key(passphrase, &salt).expect("integrity key");
    let hmac = compute_envelope_hmac(&integrity_key, "argon2id", &salt_b64, &keys).expect("hmac");

    // A V3 envelope: argon2id, a valid tag, and no `version`.
    let envelope = serde_json::json!({
        "kdf": "argon2id",
        "salt": salt_b64,
        "hmac": hmac,
        "keys": keys,
    });
    std::fs::write(&path, serde_json::to_vec(&envelope).expect("serialize")).expect("write");

    let Err(refused) = KeyStore::open(&path, passphrase) else {
        panic!("a pre-binding store must not open strictly");
    };
    assert!(
        refused.to_string().contains("per-record binding"),
        "{refused}"
    );

    let upgraded = KeyStore::open_and_migrate(&path, passphrase).expect("upgrade");
    assert_eq!(
        upgraded.load_key("iss").expect("load").fingerprint,
        fingerprint,
        "the key survives the upgrade unchanged"
    );
    drop(upgraded);
    KeyStore::open(&path, passphrase).expect("opens strictly once upgraded");
}

/// A record's ciphertext cannot be grafted onto another record's identity.
///
/// The attack the associated data blocks: take the encrypted private key from
/// one record and file it under another record's `verifying_key_hex`,
/// `fingerprint` and `revoked` flag. Before binding, that decrypted cleanly and
/// the store happily reported the wrong public key — and, worse, the wrong
/// revocation state — for that private key. Now the AEAD refuses it.
///
/// The envelope HMAC also catches this, and both are wanted: the HMAC covers the
/// map as a whole, the binding covers each record on its own, and a control that
/// only works in aggregate fails differently from one that works per record.
#[test]
fn a_record_ciphertext_cannot_be_moved_onto_another_identity() {
    let (store, path) = temp_store_at("graft", "pw");
    store.generate_key("alice").expect("generate alice");
    store.generate_key("bob").expect("generate bob");
    drop(store);

    let raw = std::fs::read_to_string(&path).expect("read");
    let mut envelope: serde_json::Value = serde_json::from_str(&raw).expect("parse");
    let alice = envelope["keys"]["alice"].clone();
    let bob = envelope["keys"]["bob"].clone();

    // Bob's plaintext identity, Alice's encrypted private key.
    let mut grafted = bob.clone();
    grafted["encryptedSigningKey"] = alice["encryptedSigningKey"].clone();
    grafted["nonce"] = alice["nonce"].clone();
    envelope["keys"]["bob"] = grafted;

    // Recompute the envelope tag so the HMAC is not what rejects this — the
    // point is that the per-record binding rejects it independently.
    let keys: KeyRecordMap = serde_json::from_value(envelope["keys"].clone()).expect("keys parse");
    let salt_b64 = envelope["salt"].as_str().expect("salt").to_owned();
    let integrity_key = super::super::crypto::derive_integrity_key(
        "pw",
        &base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            salt_b64.as_str(),
        )
        .expect("salt b64"),
    )
    .expect("integrity key");
    envelope["hmac"] = serde_json::Value::String(
        compute_envelope_hmac(&integrity_key, "argon2id", &salt_b64, &keys).expect("hmac"),
    );
    std::fs::write(&path, serde_json::to_vec(&envelope).expect("serialize")).expect("write");

    let store = KeyStore::open(&path, "pw").expect("envelope is internally consistent");
    assert!(
        store.load_key("bob").is_err(),
        "a ciphertext bound to another fingerprint must not decrypt"
    );
    assert!(
        store.load_key("alice").is_ok(),
        "the untouched record still opens"
    );
}
