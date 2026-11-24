//! What [`KeyBackend`] promises, run against every backend this crate ships.
//!
//! Each scenario takes a `&dyn KeyBackend` and uses nothing else, so a backend that
//! passes it has the behaviour the signers and the key-set builders rely on. The file
//! store and the in-memory store are listed at the bottom, and a new backend is one line.

use serde_json::json;

use super::{KeyBackend, KeyStore, MemoryKeyStore};
use crate::jws::algorithm::KeyAlgorithm;

const EVERY_ALGORITHM: [KeyAlgorithm; 4] = [
    KeyAlgorithm::Ed25519,
    KeyAlgorithm::EcdsaP256,
    KeyAlgorithm::EcdsaP384,
    KeyAlgorithm::EcdsaP521,
];

fn file_store() -> KeyStore {
    crate::test_support::open_empty_store(&crate::test_support::temp_path("backend-contract"))
}

mod scenarios {
    use super::*;

    pub(super) fn every_algorithm_signs_and_verifies(backend: &dyn KeyBackend) {
        for algorithm in EVERY_ALGORITHM {
            let key_id = format!("signs-{}", algorithm.jose_alg());
            let key = backend
                .generate_key_with(&key_id, algorithm)
                .expect("generate");
            assert_eq!(key.algorithm(), algorithm);
            assert_eq!(
                backend.public_key(&key_id).expect("recorded").algorithm,
                algorithm
            );

            let signature = key.sign(b"message").expect("sign");
            assert_eq!(signature.len(), algorithm.signature_len());
            assert!(key.public_key().verify(b"message", &signature));

            let jws = crate::jws::sign(backend, &key_id, &json!({"n": 1}), "kid").expect("a JWS");
            assert!(crate::jws::verify(backend, &key_id, &jws).expect("verify"));
        }
    }

    pub(super) fn an_unknown_key_is_not_there(backend: &dyn KeyBackend) {
        assert!(!backend.has_key("absent"));
        assert!(backend.public_key("absent").is_none());
        assert!(backend.load_key("absent").is_err());
        assert!(backend.archived_public_keys("absent").is_empty());
        assert!(backend.list().expect("list").is_empty());
    }

    pub(super) fn the_key_loaded_is_the_key_published(backend: &dyn KeyBackend) {
        backend.generate_key("k").expect("generate");
        let loaded = backend.load_key("k").expect("load");
        let published = backend.public_key("k").expect("recorded");
        assert_eq!(published.public_key().expect("a key"), loaded.public_key());
        assert!(!published.revoked && !loaded.revoked);
        assert!(backend.has_key("k"));
    }

    /// A key a caller holds is that key until the caller drops it. A rotation underneath
    /// it changes what the backend hands out next, and nothing about what it already did.
    pub(super) fn an_entry_stays_the_key_it_was_loaded_as(backend: &dyn KeyBackend) {
        backend.generate_key("k").expect("generate");
        let held = backend.load_key("k").expect("load");
        let before = held.public_key();

        backend.rotate_key("k").expect("rotate");

        assert_eq!(held.public_key(), before, "the held key did not change");
        let signature = held.sign(b"message").expect("a held key still signs");
        assert!(before.verify(b"message", &signature), "and signs as itself");
        assert_ne!(backend.load_key("k").expect("load").public_key(), before);
    }

    pub(super) fn a_rotation_archives_the_old_key_and_installs_a_new_one(backend: &dyn KeyBackend) {
        let first = backend.generate_key("k").expect("generate");
        let signature = first.sign(b"before").expect("sign");

        let second = backend.rotate_key("k").expect("rotate");

        assert_ne!(second.public_key(), first.public_key());
        assert_eq!(
            backend.load_key("k").expect("load").public_key(),
            second.public_key()
        );
        let archived = backend.archived_public_keys("k");
        assert_eq!(archived.len(), 1);
        let kept = archived[0].public_key().expect("a key");
        assert_eq!(kept, first.public_key());
        assert!(!archived[0].revoked);
        assert!(
            kept.verify(b"before", &signature),
            "a signature made before the rotation verifies against the archive"
        );
        assert!(!second.public_key().verify(b"before", &signature));
    }

    pub(super) fn archives_come_oldest_first(backend: &dyn KeyBackend) {
        let mut published = vec![backend.generate_key("k").expect("generate").public_key()];
        for _ in 0..3 {
            published.push(backend.rotate_key("k").expect("rotate").public_key());
        }
        let current = published.pop().expect("the current key");

        let archived: Vec<_> = backend
            .archived_public_keys("k")
            .iter()
            .map(|k| k.public_key().expect("a key"))
            .collect();
        assert_eq!(archived, published, "the replaced keys, oldest first");
        assert_eq!(backend.load_key("k").expect("load").public_key(), current);
    }

    pub(super) fn a_rotation_keeps_the_algorithm_unless_told_otherwise(backend: &dyn KeyBackend) {
        backend
            .generate_key_with("k", KeyAlgorithm::EcdsaP256)
            .expect("generate");
        assert_eq!(
            backend.rotate_key("k").expect("rotate").algorithm(),
            KeyAlgorithm::EcdsaP256
        );
        let moved = backend
            .rotate_key_to("k", KeyAlgorithm::Ed25519)
            .expect("rotate to another algorithm");
        assert_eq!(moved.algorithm(), KeyAlgorithm::Ed25519);

        let archived: Vec<_> = backend
            .archived_public_keys("k")
            .iter()
            .map(|k| k.algorithm)
            .collect();
        assert_eq!(
            archived,
            [KeyAlgorithm::EcdsaP256, KeyAlgorithm::EcdsaP256],
            "an archive keeps its own algorithm"
        );

        let fresh = backend.rotate_key("new").expect("rotate a key not there");
        assert_eq!(fresh.algorithm(), KeyAlgorithm::Ed25519);
        assert!(backend.archived_public_keys("new").is_empty());
    }

    pub(super) fn a_compromise_archives_the_old_key_revoked(backend: &dyn KeyBackend) {
        let a = backend.generate_key("k").expect("generate");
        let b = backend.rotate_key("k").expect("rotate");
        let c = backend.revoke_and_rotate("k").expect("revoke and rotate");

        let archived = backend.archived_public_keys("k");
        assert_eq!(archived.len(), 2);
        assert_eq!(archived[0].public_key().expect("a key"), a.public_key());
        assert!(!archived[0].revoked, "the earlier key was not compromised");
        assert_eq!(archived[1].public_key().expect("a key"), b.public_key());
        assert!(archived[1].revoked, "the key that was current was");

        let current = backend.load_key("k").expect("load");
        assert_eq!(current.public_key(), c.public_key());
        assert!(!current.revoked);
        assert!(current.sign(b"message").is_ok());
    }

    pub(super) fn generating_replaces_and_archives_nothing(backend: &dyn KeyBackend) {
        let first = backend.generate_key("k").expect("generate");
        let second = backend.generate_key("k").expect("generate again");
        assert_ne!(first.public_key(), second.public_key());
        assert_eq!(
            backend.load_key("k").expect("load").public_key(),
            second.public_key()
        );
        assert!(backend.archived_public_keys("k").is_empty());
    }

    pub(super) fn the_list_names_the_current_keys_by_id(backend: &dyn KeyBackend) {
        backend
            .generate_key_with("b", KeyAlgorithm::EcdsaP384)
            .expect("generate");
        let a = backend.generate_key("a").expect("generate");
        backend.rotate_key("a").expect("rotate");
        let a_now = backend.load_key("a").expect("load");

        let listed = backend.list().expect("list");
        let ids: Vec<&str> = listed.iter().map(|(id, _)| id.as_str()).collect();
        assert_eq!(ids, ["a", "b"], "sorted, and without the archive");
        assert_eq!(
            listed[0].1.public_key().expect("a key"),
            a_now.public_key(),
            "the current key, not the archived one"
        );
        assert_ne!(listed[0].1.public_key().expect("a key"), a.public_key());
        assert_eq!(listed[1].1.algorithm, KeyAlgorithm::EcdsaP384);
        assert!(!listed[1].1.revoked);
    }
}

macro_rules! contract_on {
    ($backend:ident, $make:expr) => {
        mod $backend {
            use super::*;

            #[test]
            fn every_algorithm_signs_and_verifies() {
                scenarios::every_algorithm_signs_and_verifies(&$make);
            }

            #[test]
            fn an_unknown_key_is_not_there() {
                scenarios::an_unknown_key_is_not_there(&$make);
            }

            #[test]
            fn the_key_loaded_is_the_key_published() {
                scenarios::the_key_loaded_is_the_key_published(&$make);
            }

            #[test]
            fn an_entry_stays_the_key_it_was_loaded_as() {
                scenarios::an_entry_stays_the_key_it_was_loaded_as(&$make);
            }

            #[test]
            fn a_rotation_archives_the_old_key_and_installs_a_new_one() {
                scenarios::a_rotation_archives_the_old_key_and_installs_a_new_one(&$make);
            }

            #[test]
            fn archives_come_oldest_first() {
                scenarios::archives_come_oldest_first(&$make);
            }

            #[test]
            fn a_rotation_keeps_the_algorithm_unless_told_otherwise() {
                scenarios::a_rotation_keeps_the_algorithm_unless_told_otherwise(&$make);
            }

            #[test]
            fn a_compromise_archives_the_old_key_revoked() {
                scenarios::a_compromise_archives_the_old_key_revoked(&$make);
            }

            #[test]
            fn generating_replaces_and_archives_nothing() {
                scenarios::generating_replaces_and_archives_nothing(&$make);
            }

            #[test]
            fn the_list_names_the_current_keys_by_id() {
                scenarios::the_list_names_the_current_keys_by_id(&$make);
            }
        }
    };
}

contract_on!(file_backend, file_store());
contract_on!(memory_backend, MemoryKeyStore::new());
