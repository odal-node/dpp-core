use std::collections::HashMap;

use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, consts::U12},
};
use ed25519_dalek::SigningKey;
use rand::Rng;
use sha2::{Digest, Sha256};
use zeroize::Zeroize;

use super::crypto::{compute_envelope_hmac, derive_aes_key_sha256};
use super::store::{KeyRecord, KeyRecordMap, KeyStore, default_algorithm};

fn temp_store() -> KeyStore {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("test-keystore-{}.json", uuid::Uuid::now_v7()));
    KeyStore::open(path, "test-passphrase").expect("open key store")
}

#[test]
fn generate_then_load_roundtrip() {
    let store = temp_store();
    let generated = store.generate_key("issuer-1").expect("generate key");
    let loaded = store.load_key("issuer-1").expect("load key");
    assert_eq!(generated.fingerprint, loaded.fingerprint);
    assert_eq!(
        generated.verifying_key.as_bytes(),
        loaded.verifying_key.as_bytes()
    );
}

#[test]
fn stored_bytes_differ_from_plaintext() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("test-keystore-enc-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(&path, "test-passphrase").expect("open");
    let key = store.generate_key("issuer-enc").expect("generate");

    let raw_file = std::fs::read_to_string(&path).expect("read file");
    let plaintext_hex = hex::encode(key.signing_key.as_bytes());
    assert!(
        !raw_file.contains(&plaintext_hex),
        "plaintext key bytes must not appear in the store file"
    );
}

#[test]
fn store_file_contains_argon2id_kdf_marker() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("test-keystore-kdf-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(&path, "test-passphrase").expect("open");
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
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "test-keystore-reopen-{}.json",
        uuid::Uuid::now_v7()
    ));
    let store = KeyStore::open(&path, "my-passphrase").expect("open");
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
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "test-keystore-legacy-{}.json",
        uuid::Uuid::now_v7()
    ));

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

    // Open the legacy store — should succeed.
    let store = KeyStore::open(&path, passphrase).expect("open legacy store");
    let loaded = store.load_key("legacy-key").expect("load legacy key");
    assert_eq!(loaded.fingerprint, fingerprint);

    // Migrate.
    store
        .migrate_if_needed(passphrase)
        .expect("migration failed");

    // Verify the file now contains the argon2id marker.
    let raw_file = std::fs::read_to_string(&path).expect("read");
    assert!(raw_file.contains("argon2id"), "file must be migrated");
}

#[test]
fn archive_key_creates_archived_entry() {
    let store = temp_store();
    store.generate_key("issuer-arc").expect("generate");
    store.archive_key("issuer-arc").expect("archive");
    let archived = store.load_archived_keys("issuer-arc");
    assert_eq!(archived.len(), 1, "expected one archived key");
}

#[test]
fn load_archived_keys_empty_before_rotation() {
    let store = temp_store();
    store.generate_key("issuer-noarc").expect("generate");
    let archived = store.load_archived_keys("issuer-noarc");
    assert!(
        archived.is_empty(),
        "no archived keys before first rotation"
    );
}

// ── Gap 7: atomic rotation + revocation ───────────────────────────────────

#[test]
fn rotate_key_archives_old_and_installs_new() {
    let store = temp_store();
    let k1 = store.generate_key("iss").expect("gen");
    let k2 = store.rotate_key("iss").expect("rotate");
    assert_ne!(
        k1.fingerprint, k2.fingerprint,
        "rotation installs a new key"
    );
    assert_eq!(
        store.load_key("iss").unwrap().fingerprint,
        k2.fingerprint,
        "current key is the new one"
    );
    let archived = store.load_archived_keys("iss");
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].fingerprint, k1.fingerprint);
    assert!(
        !archived[0].revoked,
        "hygiene rotation keeps the old key valid"
    );
}

#[test]
fn revoke_and_rotate_marks_old_key_revoked() {
    let store = temp_store();
    let k1 = store.generate_key("iss").expect("gen");
    let k2 = store.revoke_and_rotate("iss").expect("revoke+rotate");
    assert_ne!(k1.fingerprint, k2.fingerprint);
    assert!(!k2.revoked, "the new current key is not revoked");
    let archived = store.load_archived_keys("iss");
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].fingerprint, k1.fingerprint);
    assert!(
        archived[0].revoked,
        "compromise rotation marks the old key revoked"
    );
}

#[test]
fn revoked_state_persists_across_reopen() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "test-keystore-revoke-{}.json",
        uuid::Uuid::now_v7()
    ));
    let store = KeyStore::open(&path, "rev").expect("open");
    store.generate_key("iss").expect("gen");
    let revoked_fp = store.load_key("iss").unwrap().fingerprint.clone();
    store.revoke_and_rotate("iss").expect("revoke+rotate");
    drop(store);

    let store2 = KeyStore::open(&path, "rev").expect("reopen");
    let archived = store2.load_archived_keys("iss");
    assert!(
        archived
            .iter()
            .any(|k| k.fingerprint == revoked_fp && k.revoked),
        "revoked flag must survive a reopen (and the HMAC must still verify)"
    );
}

#[test]
fn store_file_contains_hmac() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!("test-keystore-hmac-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(&path, "hmac-test").expect("open");
    store.generate_key("issuer-hmac").expect("generate");

    let raw = std::fs::read_to_string(&path).expect("read file");
    assert!(
        raw.contains("\"hmac\""),
        "store file must contain HMAC field"
    );

    let envelope: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let hmac_val = envelope["hmac"].as_str().unwrap();
    // HMAC-SHA256 produces 64 hex characters
    assert_eq!(hmac_val.len(), 64, "HMAC must be 64 hex chars");
}

#[test]
fn tampered_store_file_rejected_on_open() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "test-keystore-tamper-{}.json",
        uuid::Uuid::now_v7()
    ));
    let store = KeyStore::open(&path, "tamper-test").expect("open");
    store.generate_key("issuer-tamper").expect("generate");
    drop(store);

    // Tamper with the file: change a fingerprint value.
    let mut raw = std::fs::read_to_string(&path).expect("read");
    // Replace first hex char of any fingerprint
    if let Some(pos) = raw.find("\"fingerprint\":\"") {
        let fp_start = pos + "\"fingerprint\":\"".len();
        let old_char = raw.as_bytes()[fp_start];
        let new_char = if old_char == b'a' { b'b' } else { b'a' };
        unsafe {
            raw.as_bytes_mut()[fp_start] = new_char;
        }
    }
    std::fs::write(&path, &raw).expect("write tampered file");

    // Re-opening should fail due to HMAC mismatch.
    let result = KeyStore::open(&path, "tamper-test");
    assert!(
        result.is_err(),
        "tampered store should fail integrity check"
    );
    let Err(e) = result else {
        panic!("tampered store should fail integrity check");
    };
    let err = e.to_string();
    assert!(
        err.contains("integrity") || err.contains("tamper"),
        "error should mention integrity, got: {err}"
    );
}

#[test]
fn reopen_with_hmac_succeeds() {
    let dir = std::env::temp_dir();
    let path = dir.join(format!(
        "test-keystore-hmac-ok-{}.json",
        uuid::Uuid::now_v7()
    ));
    let store = KeyStore::open(&path, "hmac-ok").expect("open");
    store.generate_key("issuer-ok").expect("generate");
    drop(store);

    // Re-open — should pass HMAC verification.
    let store2 = KeyStore::open(&path, "hmac-ok").expect("reopen");
    let loaded = store2.load_key("issuer-ok").expect("load");
    assert!(!loaded.fingerprint.is_empty());
}

/// Regression: HMAC must be identical regardless of HashMap insertion order.
/// Before the BTreeMap fix, two HashMaps with the same entries but different
/// insertion orders could iterate in different sequences and produce different
/// HMAC digests, falsely rejecting a valid store as tampered after restart.
#[test]
fn hmac_is_stable_across_map_insertion_order() {
    let integrity_key = [42u8; 32];

    let make_record = |n: u8| KeyRecord {
        encrypted_signing_key: vec![n; 48],
        nonce: vec![n; 12],
        fingerprint: format!("fp{n:02x}"),
        verifying_key_hex: format!("{:064x}", n),
        revoked: false,
        algorithm: default_algorithm(),
    };

    let keys_fwd = ["zebra", "alpha", "mango", "delta", "beta"];

    let mut map_a: KeyRecordMap = HashMap::new();
    for (i, k) in keys_fwd.iter().enumerate() {
        map_a.insert(k.to_string(), make_record(i as u8));
    }

    // Rebuild with the same key→value mapping but drained and re-inserted
    // in reverse order to try to get a different HashMap layout.
    let mut map_b: KeyRecordMap = HashMap::new();
    for (i, k) in keys_fwd.iter().enumerate() {
        map_b.insert(k.to_string(), make_record(i as u8));
    }
    let entries: Vec<_> = map_b.drain().collect();
    let mut map_b: KeyRecordMap = HashMap::new();
    for (k, v) in entries.into_iter().rev() {
        map_b.insert(k, v);
    }

    let test_salt = "dGVzdHNhbHQ="; // base64("testsalt")
    let hmac_a = compute_envelope_hmac(&integrity_key, "argon2id", test_salt, &map_a).unwrap();
    let hmac_b = compute_envelope_hmac(&integrity_key, "argon2id", test_salt, &map_b).unwrap();

    assert_eq!(
        hmac_a, hmac_b,
        "HMAC must be identical regardless of HashMap insertion/iteration order"
    );
}
