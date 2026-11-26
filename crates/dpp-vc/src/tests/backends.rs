//! Rotation, revocation and the published key sets, through [`KeyBackend`] alone, on
//! every backend `dpp-crypto` ships.
//!
//! `rotation.rs` proves these for the file store with the file store's own methods. A
//! second backend that passes the same scenarios, reached only through the trait, is what
//! shows the trait is enough for everything that signs and publishes.

use std::sync::Arc;

use serde_json::{Value, json};

use dpp_crypto::keystore::{KeyBackend, MemoryKeyStore};
use dpp_domain::PassportId;
use dpp_domain::ports::identity::IdentityPort;

use super::common::{kid_named, verifies_in, without_waiting};
use crate::did_builder::{build_did_document, did_for};
use crate::local_service::LocalIdentityService;
use crate::sd_jwt_vc::build_issuer_metadata;
use crate::test_support::EVERY_ALGORITHM;

const BASE_URL: &str = "https://id.example.com";
const KEY_ID: &str = "issuer";

/// The file store, with the key generated.
fn file_backend() -> Arc<dyn KeyBackend> {
    Arc::new(crate::test_support::temp_store("backends", KEY_ID))
}

/// The in-memory backend, with the key generated.
fn memory_backend() -> Arc<dyn KeyBackend> {
    let store = MemoryKeyStore::new();
    store.generate_key(KEY_ID).expect("generate the key");
    Arc::new(store)
}

/// Sign as a passport is signed: the `kid` is the DID URL of the verification method for
/// the key that is current now.
fn sign_as_current(store: &dyn KeyBackend, payload: &Value) -> String {
    let thumbprint = store
        .public_key(KEY_ID)
        .expect("a current key")
        .thumbprint_uri()
        .expect("thumbprint");
    let kid = format!("{}#{thumbprint}", did_for(BASE_URL));
    dpp_crypto::jws::signer::sign(store, KEY_ID, payload, &kid).expect("sign")
}

fn did_document(store: &dyn KeyBackend) -> Value {
    build_did_document(store, BASE_URL, KEY_ID).expect("build the document")
}

/// How many keys the SD-JWT VC issuer metadata publishes, or `None` when it publishes none.
fn published_keys(store: &dyn KeyBackend) -> Option<usize> {
    let metadata = build_issuer_metadata(store, BASE_URL, KEY_ID)?;
    Some(
        metadata["jwks"]["keys"]
            .as_array()
            .expect("a key set")
            .len(),
    )
}

mod scenarios {
    use super::*;

    pub(super) fn rotation_does_not_break_old_jws_verification(store: &dyn KeyBackend) {
        let payload = json!({"product": "battery", "status": "draft"});
        let jws_a = sign_as_current(store, &payload);
        let first_id = did_document(store)["verificationMethod"][0]["id"].clone();

        store.rotate_key(KEY_ID).expect("rotate");
        let doc = did_document(store);

        let methods = doc["verificationMethod"].as_array().expect("methods");
        assert_eq!(methods.len(), 2, "the document lists both keys");
        assert!(
            methods.iter().any(|m| m["id"] == first_id),
            "the first key keeps the identifier it had when it was current"
        );
        assert!(verifies_in(&doc, &jws_a), "the old token verifies");
        assert!(
            verifies_in(&doc, &sign_as_current(store, &payload)),
            "the new token verifies"
        );
    }

    pub(super) fn a_revoked_keys_signature_no_longer_verifies(store: &dyn KeyBackend) {
        let payload = json!({"product": "battery", "status": "draft"});
        let jws_a = sign_as_current(store, &payload);

        store.revoke_and_rotate(KEY_ID).expect("revoke and rotate");
        let doc = did_document(store);

        assert!(
            dpp_crypto::jws::verifier::resolve_verification_key(&doc, &jws_a).is_none(),
            "a revoked key cannot be selected for verification"
        );
        assert!(
            verifies_in(&doc, &sign_as_current(store, &payload)),
            "the new current key resolves"
        );
    }

    pub(super) fn a_revoked_key_dropping_out_does_not_rename_the_others(store: &dyn KeyBackend) {
        let payload = json!({"product": "battery", "status": "draft"});
        let jws_first = sign_as_current(store, &payload);
        let first_id = did_document(store)["verificationMethod"][0]["id"].clone();

        // The first key is archived by hygiene, the second revoked on compromise.
        store.rotate_key(KEY_ID).expect("archive the first key");
        store.revoke_and_rotate(KEY_ID).expect("revoke the second");
        let doc = did_document(store);

        let methods = doc["verificationMethod"].as_array().expect("methods");
        assert_eq!(methods.len(), 2, "the current key and the first");
        assert!(methods.iter().any(|m| m["id"] == first_id));
        assert!(verifies_in(&doc, &jws_first));
    }

    pub(super) fn a_rotation_across_algorithms_keeps_every_signature_verifiable(
        store: &dyn KeyBackend,
    ) {
        let payload = json!({"product": "battery", "status": "draft"});
        let mut tokens = vec![sign_as_current(store, &payload)];
        for target in EVERY_ALGORITHM {
            store.rotate_key_to(KEY_ID, target).expect("rotate");
            tokens.push(sign_as_current(store, &payload));
        }

        let doc = did_document(store);
        assert_eq!(
            doc["verificationMethod"].as_array().expect("methods").len(),
            1 + EVERY_ALGORITHM.len(),
            "every key that signed is published"
        );
        for (i, jws) in tokens.iter().enumerate() {
            assert!(verifies_in(&doc, jws), "token {i} verifies");
        }
    }

    pub(super) fn the_issuer_metadata_lists_the_archive_and_drops_the_revoked(
        store: &dyn KeyBackend,
    ) {
        assert_eq!(published_keys(store), Some(1), "the current key");
        store.rotate_key(KEY_ID).expect("rotate");
        assert_eq!(
            published_keys(store),
            Some(2),
            "the archive and the current"
        );
        store.revoke_and_rotate(KEY_ID).expect("revoke and rotate");
        assert_eq!(
            published_keys(store),
            Some(2),
            "the revoked key is dropped, the first archive and the current stay"
        );
    }

    pub(super) fn a_key_set_is_not_published_for_a_key_that_is_not_there(store: &dyn KeyBackend) {
        assert!(build_issuer_metadata(store, BASE_URL, "absent").is_none());
        assert!(!store.has_key("absent"), "asking for it made no key");
    }

    pub(super) fn passports_signed_while_the_key_rotates_verify_after(store: Arc<dyn KeyBackend>) {
        let service =
            LocalIdentityService::new(Arc::clone(&store), KEY_ID.to_owned(), BASE_URL.to_owned());
        let payload = json!({"product": "battery", "status": "draft"});

        let passports = crate::test_support::sign_while_rotating(&*store, KEY_ID, || {
            without_waiting(service.sign_passport(PassportId::new(), &payload))
                .expect("sign while the key rotates")
                .jws
        });
        let doc = did_document(&*store);

        let kids: std::collections::BTreeSet<String> =
            passports.iter().map(|jws| kid_named(jws)).collect();
        assert!(
            kids.len() > 1,
            "the passports were signed under more than one key"
        );
        for (i, jws) in passports.iter().enumerate() {
            assert!(
                verifies_in(&doc, jws),
                "passport {i} names {} and does not verify under it",
                kid_named(jws)
            );
        }
    }

    pub(super) fn a_key_of_every_algorithm_is_published_under_its_own_shape(
        store: &dyn KeyBackend,
    ) {
        for algorithm in EVERY_ALGORITHM {
            let key_id = format!("shape-{}", algorithm.jose_alg());
            store
                .generate_key_with(&key_id, algorithm)
                .expect("generate");
            let doc = build_did_document(store, BASE_URL, &key_id).expect("build");
            let jwk = &doc["verificationMethod"][0]["publicKeyJwk"];
            let published = dpp_crypto::jws::PublicKey::from_jwk(jwk).expect("a public key JWK");
            assert_eq!(published.algorithm(), algorithm);
            assert_eq!(
                published,
                store.load_key(&key_id).expect("load").public_key(),
                "the document publishes the key the backend signs with"
            );
        }
    }
}

macro_rules! scenarios_on {
    ($backend:ident, $make:expr) => {
        mod $backend {
            use super::*;

            #[test]
            fn rotation_does_not_break_old_jws_verification() {
                scenarios::rotation_does_not_break_old_jws_verification(&*$make);
            }

            #[test]
            fn a_revoked_keys_signature_no_longer_verifies() {
                scenarios::a_revoked_keys_signature_no_longer_verifies(&*$make);
            }

            #[test]
            fn a_revoked_key_dropping_out_does_not_rename_the_others() {
                scenarios::a_revoked_key_dropping_out_does_not_rename_the_others(&*$make);
            }

            #[test]
            fn a_rotation_across_algorithms_keeps_every_signature_verifiable() {
                scenarios::a_rotation_across_algorithms_keeps_every_signature_verifiable(&*$make);
            }

            #[test]
            fn the_issuer_metadata_lists_the_archive_and_drops_the_revoked() {
                scenarios::the_issuer_metadata_lists_the_archive_and_drops_the_revoked(&*$make);
            }

            #[test]
            fn a_key_set_is_not_published_for_a_key_that_is_not_there() {
                scenarios::a_key_set_is_not_published_for_a_key_that_is_not_there(&*$make);
            }

            #[test]
            fn passports_signed_while_the_key_rotates_verify_after() {
                scenarios::passports_signed_while_the_key_rotates_verify_after($make);
            }

            #[test]
            fn a_key_of_every_algorithm_is_published_under_its_own_shape() {
                scenarios::a_key_of_every_algorithm_is_published_under_its_own_shape(&*$make);
            }
        }
    };
}

scenarios_on!(file_store, file_backend());
scenarios_on!(memory_store, memory_backend());
