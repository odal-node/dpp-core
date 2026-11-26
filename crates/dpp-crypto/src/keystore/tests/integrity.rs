//! The integrity tag over a store file.

use std::collections::HashMap;

use super::super::crypto::compute_envelope_hmac;
use super::super::store::{KeyRecord, KeyRecordMap, KeyStore, default_algorithm};
use super::common::*;

#[test]
fn store_file_contains_hmac() {
    let (store, path) = temp_store_at("hmac", "hmac-test");
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
    let (store, path) = temp_store_at("tamper", "tamper-test");
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
    let (store, path) = temp_store_at("hmac-ok", "hmac-ok");
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

// ── algorithm agility ─────────────────────────────────────────────────────────
