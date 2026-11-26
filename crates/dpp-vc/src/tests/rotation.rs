//! Key rotation across the DID document.
//!
//! These moved from `dpp-crypto`'s JWS suite when the `did:web` builder moved
//! here: they assert a property that spans both crates — a JWS signed before a
//! rotation still verifies against the DID document afterwards, and a revoked
//! key's signature does not. `dpp-vc` is where they can see both halves.

use std::sync::Arc;

use base64::Engine;
use serde_json::json;

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::{KeyEntry, KeyStore};
use dpp_domain::PassportId;
use dpp_domain::ports::identity::IdentityPort;

use super::common::{kid_named, verifies_in, without_waiting};
use crate::local_service::LocalIdentityService;
use crate::test_support::{EVERY_ALGORITHM, temp_store_with};

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

/// The identifier the current key is published under.
fn current_kid(store: &KeyStore) -> String {
    let thumbprint = store
        .public_key("issuer")
        .expect("a current key")
        .thumbprint_uri()
        .expect("thumbprint");
    format!(
        "{}#{thumbprint}",
        crate::did_builder::did_for(ROTATION_BASE_URL)
    )
}

/// A compact JWS over `payload`, signed by `key` and naming `kid`. Assembled here so
/// that a test can sign with a key the store no longer makes current.
fn sign_with(key: &KeyEntry, payload: &serde_json::Value, kid: &str) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = json!({ "alg": key.algorithm().jose_alg(), "kid": kid });
    let body = dpp_crypto::jws::canonicalize(payload).expect("canonical payload");
    let signing_input = format!("{}.{}", b64.encode(header.to_string()), b64.encode(body));
    let signature = key
        .sign(signing_input.as_bytes())
        .expect("an unrevoked key signs");
    format!("{signing_input}.{}", b64.encode(signature))
}

/// A rotation from Ed25519 to an EC key keeps a signature made before it verifiable,
/// and one made after it, for each curve the issuer may move to.
#[test]
fn a_rotation_from_ed25519_to_an_ec_key_keeps_every_signature_verifiable() {
    for target in [
        KeyAlgorithm::EcdsaP256,
        KeyAlgorithm::EcdsaP384,
        KeyAlgorithm::EcdsaP521,
    ] {
        let label = target.jose_alg();
        let store = rotation_store(&format!("ed-to-{label}"));
        let payload = json!({"product": "battery", "status": "draft"});
        let before = sign_as_current(&store, &payload);

        store
            .rotate_key_to("issuer", target)
            .expect("rotate to an EC key");
        let after = sign_as_current(&store, &payload);
        let doc = did_document(&store);

        assert!(
            verifies_in(&doc, &before),
            "{label}: a signature made before the rotation must verify"
        );
        assert!(
            verifies_in(&doc, &after),
            "{label}: a signature made after the rotation must verify"
        );
    }
}

/// A passport signed before a rotation to an EC key verifies after it, and so does
/// one signed after, through the service that signs passports.
#[tokio::test]
async fn a_passport_signed_before_a_rotation_to_an_ec_key_still_verifies() {
    let store = Arc::new(rotation_store("passport-rotate"));
    let svc = LocalIdentityService::new(
        store.clone(),
        "issuer".to_owned(),
        ROTATION_BASE_URL.to_owned(),
    );
    let payload = json!({"productName": "Widget", "productGroup": "battery"});
    let before = svc
        .sign_passport(PassportId::new(), &payload)
        .await
        .expect("sign before the rotation");

    store
        .rotate_key_to("issuer", KeyAlgorithm::EcdsaP256)
        .expect("rotate to P-256");
    let after = svc
        .sign_passport(PassportId::new(), &payload)
        .await
        .expect("sign after the rotation");

    for (when, jws) in [("before", &before.jws), ("after", &after.jws)] {
        assert!(
            svc.verify_signature(jws, &payload).await.expect("verify"),
            "a passport signed {when} the rotation must verify"
        );
    }
}

/// A token whose `kid` names the other key of its own document is refused, whichever
/// key made it, and whether or not the two keys share an algorithm. The `kid` is the
/// key the verifier checks against, so a signature by one key under the other's name
/// is a signature by no key the document lists under that name.
#[test]
fn a_token_whose_kid_names_the_other_key_of_its_document_is_refused() {
    for old in EVERY_ALGORITHM {
        for new in EVERY_ALGORITHM {
            let label = format!("swap-{}-{}", old.jose_alg(), new.jose_alg());
            let store = temp_store_with(&label, "issuer", old);
            let payload = json!({"product": "battery", "status": "draft"});
            let old_kid = current_kid(&store);
            store.rotate_key_to("issuer", new).expect("rotate");
            let new_kid = current_kid(&store);

            let old_key = store
                .load_archived_keys("issuer")
                .into_iter()
                .next()
                .expect("the archived key");
            let new_key = store.load_key("issuer").expect("the current key");
            assert_eq!(
                old_key.algorithm(),
                old,
                "{label}: the archived key is a key of the old algorithm"
            );
            assert_eq!(
                new_key.algorithm(),
                new,
                "{label}: the current key is a key of the new algorithm"
            );
            let doc = did_document(&store);

            assert!(
                verifies_in(&doc, &sign_with(&old_key, &payload, &old_kid)),
                "{label}: a token under its own kid verifies"
            );
            assert!(
                verifies_in(&doc, &sign_with(&new_key, &payload, &new_kid)),
                "{label}: a token under its own kid verifies"
            );
            assert!(
                !verifies_in(&doc, &sign_with(&old_key, &payload, &new_kid)),
                "{label}: the archived key's signature, under the current key's kid, must not verify"
            );
            assert!(
                !verifies_in(&doc, &sign_with(&new_key, &payload, &old_kid)),
                "{label}: the current key's signature, under the archived key's kid, must not verify"
            );
        }
    }
}

/// A passport signed while its key rotates names the key that signed it, and
/// verifies against the DID document published once the rotations stop.
///
/// `sign_passport` takes the `kid` and the signature from one load of the key. With
/// two reads, a rotation between them gave a passport the replaced key's `kid` and
/// the new key's signature, and the document refused it.
#[test]
fn passports_signed_while_the_key_rotates_verify_against_the_document_published_after() {
    let store = Arc::new(rotation_store("passport-while-rotating"));
    let svc = LocalIdentityService::new(
        Arc::<KeyStore>::clone(&store),
        "issuer".to_owned(),
        ROTATION_BASE_URL.to_owned(),
    );
    let payload = json!({"product": "battery", "status": "draft"});

    let passports = crate::test_support::sign_while_rotating(&store, "issuer", || {
        without_waiting(svc.sign_passport(PassportId::new(), &payload))
            .expect("sign while the key rotates")
            .jws
    });
    let doc = did_document(&store);

    let kids: std::collections::BTreeSet<String> =
        passports.iter().map(|jws| kid_named(jws)).collect();
    assert!(
        kids.len() > 1,
        "the passports were signed under more than one key: {}",
        kids.len()
    );
    for (i, jws) in passports.iter().enumerate() {
        assert!(
            verifies_in(&doc, jws),
            "passport {i} of {} names {} and does not verify under it",
            passports.len(),
            kid_named(jws)
        );
    }
}
