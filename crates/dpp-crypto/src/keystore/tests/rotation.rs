//! Archiving, rotating and revoking a key.

use super::super::store::KeyStore;
use super::common::*;

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

// ── Atomic rotation + revocation ───────────────────────────────────

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

/// An archive taken before a compromise rotation holds the same key material, so
/// the rotation must revoke it in the same write. Left unrevoked, it stays in the
/// published DID document and the compromised key keeps verifying.
#[test]
fn compromise_rotation_revokes_earlier_archives_of_the_same_key() {
    let store = temp_store();
    let k1 = store.generate_key("iss").expect("gen");
    store.archive_key("iss").expect("archive");
    store.revoke_and_rotate("iss").expect("revoke+rotate");

    let archived = store.load_archived_keys("iss");
    assert_eq!(archived.len(), 2);
    assert!(
        archived
            .iter()
            .all(|a| a.fingerprint == k1.fingerprint && a.revoked),
        "every archived copy of the compromised key is revoked"
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

/// A store whose current key under `iss` is revoked, the state a store opens in
/// when its file says so. Set in memory here; `dpp-vc`'s tests reach it the way a
/// deployment would, through a legacy file sealed by `open_and_migrate`.
fn store_with_revoked_current() -> KeyStore {
    let store = temp_store();
    store.generate_key("iss").expect("gen");
    store
        .records
        .write()
        .expect("lock")
        .get_mut("iss")
        .expect("the current record")
        .revoked = true;
    store
}

/// A revoked key signs nothing: not through `KeyEntry::sign`, not as a JWS, and the
/// JWS verifier refuses it as well.
#[test]
fn a_revoked_key_signs_nothing() {
    let store = store_with_revoked_current();
    let key = store.load_key("iss").expect("a revoked key still loads");
    assert!(key.revoked);

    let super::super::SignError::Revoked(refused) = key
        .sign(b"message")
        .expect_err("a revoked key refuses to sign")
    else {
        panic!("a revoked key is refused for its revocation");
    };
    assert_eq!(refused.fingerprint, key.fingerprint);
    assert!(
        refused.to_string().contains("revoke_and_rotate"),
        "{refused}"
    );

    let jws = crate::jws::sign(&store, "iss", &serde_json::json!({}), "kid")
        .expect_err("jws::sign refuses");
    assert_eq!(
        jws.downcast_ref::<super::super::KeyRevoked>(),
        Some(&refused)
    );
    let verify = crate::jws::verify(&store, "iss", "e30.e30.AA").expect_err("verify refuses");
    assert_eq!(
        verify.downcast_ref::<super::super::KeyRevoked>(),
        Some(&refused)
    );
}

/// A hygiene rotation of a revoked current key archives it revoked. Archiving it as
/// a valid key would publish it again and make its signatures verify.
#[test]
fn rotating_a_revoked_current_key_keeps_it_revoked() {
    let store = store_with_revoked_current();
    let revoked = store.load_key("iss").expect("load").fingerprint.clone();

    let successor = store.rotate_key("iss").expect("rotate");
    successor.sign(b"message").expect("the successor signs");
    let archived = store.load_archived_keys("iss");
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].fingerprint, revoked);
    assert!(archived[0].revoked, "the revoked key stays revoked");
}

#[test]
fn revoked_state_persists_across_reopen() {
    let (store, path) = temp_store_at("revoke", "rev");
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
