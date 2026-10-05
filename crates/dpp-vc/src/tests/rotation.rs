//! Key rotation across the DID document.
//!
//! These moved from `dpp-crypto`'s JWS suite when the `did:web` builder moved
//! here: they assert a property that spans both crates — a JWS signed before a
//! rotation still verifies against the DID document afterwards, and a revoked
//! key's signature does not. `dpp-vc` is where they can see both halves.

use serde_json::json;

use dpp_crypto::keystore::KeyStore;

const ROTATION_BASE_URL: &str = "https://id.example.com";

fn rotation_store(label: &str) -> KeyStore {
    crate::test_support::temp_store(label, "issuer")
}

/// Sign as a passport is signed: the `kid` is the DID URL of the verification
/// method for the key that is current *now*.
fn sign_as_current(store: &KeyStore, payload: &serde_json::Value) -> String {
    let thumbprint = store
        .public_key("issuer")
        .expect("a current key")
        .thumbprint_uri()
        .expect("thumbprint");
    let kid = format!(
        "{}#{thumbprint}",
        crate::did_builder::did_for(ROTATION_BASE_URL)
    );
    dpp_crypto::jws::signer::sign(store, "issuer", payload, &kid).expect("sign")
}

fn did_document(store: &KeyStore) -> serde_json::Value {
    crate::did_builder::build_did_document(store, ROTATION_BASE_URL, "issuer").expect("build")
}

/// Whether `jws` resolves in `doc` to a key it verifies under.
fn verifies_in(doc: &serde_json::Value, jws: &str) -> bool {
    dpp_crypto::jws::verifier::resolve_verification_key(doc, jws)
        .is_some_and(|key| dpp_crypto::jws::verifier::verify_jws(jws, &key).expect("verify"))
}

/// Regression (W-2): sign with key A, rotate to key B, verify old JWS against
/// a DID document that contains both keys.
#[test]
fn rotation_does_not_break_old_jws_verification() {
    let store = rotation_store("w2-rotation");
    let payload = json!({"product": "battery", "status": "draft"});
    let jws_a = sign_as_current(&store, &payload);
    let key_a_id = did_document(&store)["verificationMethod"][0]["id"].clone();

    store.archive_key("issuer").expect("archive A");
    store.generate_key("issuer").expect("generate key B");
    let did_doc = did_document(&store);

    let methods = did_doc["verificationMethod"].as_array().unwrap();
    assert_eq!(
        methods.len(),
        2,
        "DID doc must list both keys after rotation"
    );
    assert!(
        methods.iter().any(|m| m["id"] == key_a_id),
        "key A keeps the identifier it had when it was current"
    );
    assert!(
        verifies_in(&did_doc, &jws_a),
        "old JWS must verify against the archived key after rotation"
    );

    let jws_b = sign_as_current(&store, &payload);
    assert!(
        verifies_in(&did_doc, &jws_b),
        "new JWS must verify against the current key"
    );
}

/// End-to-end: after a key is **revoked**, a JWS it produced must no
/// longer be verifiable — the revoked key is absent from the DID document.
#[test]
fn revoked_key_signature_no_longer_verifies() {
    let store = rotation_store("revoke-verify");
    let payload = json!({"product": "battery", "status": "draft"});
    let jws_a = sign_as_current(&store, &payload);

    store.revoke_and_rotate("issuer").expect("revoke+rotate");
    let did_doc = did_document(&store);

    assert!(
        dpp_crypto::jws::verifier::resolve_verification_key(&did_doc, &jws_a).is_none(),
        "a revoked key must not be selectable for verification"
    );
    let jws_b = sign_as_current(&store, &payload);
    assert!(
        verifies_in(&did_doc, &jws_b),
        "the new current key resolves"
    );
}

/// What a position in a list could not do. A revoked key drops out of the
/// document, and the keys that remain keep the identifiers they had, so a token
/// issued before the revocation still names the key that signed it.
#[test]
fn a_revoked_key_dropping_out_does_not_rename_the_others() {
    let store = rotation_store("revoke-renames");
    let payload = json!({"product": "battery", "status": "draft"});
    let jws_first = sign_as_current(&store, &payload);
    let first_id = did_document(&store)["verificationMethod"][0]["id"].clone();

    // The first key is archived by hygiene, the second revoked on compromise.
    store.archive_key("issuer").expect("archive the first key");
    store
        .generate_key("issuer")
        .expect("generate the second key");
    store
        .revoke_and_rotate("issuer")
        .expect("revoke the second key");
    let did_doc = did_document(&store);

    let methods = did_doc["verificationMethod"].as_array().unwrap();
    assert_eq!(
        methods.len(),
        2,
        "the current key and the first, not the revoked"
    );
    assert!(methods.iter().any(|m| m["id"] == first_id));
    assert!(
        verifies_in(&did_doc, &jws_first),
        "a token issued under the first key still resolves after another key was revoked"
    );
}
