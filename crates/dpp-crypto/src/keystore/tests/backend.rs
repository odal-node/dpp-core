//! What [`KeyBackend`] promises, run against every backend this crate ships.
//!
//! Each scenario takes a `&dyn KeyBackend` and uses nothing else, so a backend that
//! passes it has the behaviour the signers and the key-set builders rely on. The file
//! store and the in-memory store are listed at the bottom, and a new backend is one line.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use serde_json::json;

use super::super::{KeyBackend, KeyEntry, KeyStore, MemoryKeyStore, PublicKeyInfo};
use crate::jws::algorithm::KeyAlgorithm;

const EVERY_ALGORITHM: [KeyAlgorithm; 4] = [
    KeyAlgorithm::Ed25519,
    KeyAlgorithm::EcdsaP256,
    KeyAlgorithm::EcdsaP384,
    KeyAlgorithm::EcdsaP521,
];

/// A directory removed when it drops, with the store file and the lock file beside it.
struct TempDir(PathBuf);

impl Drop for TempDir {
    fn drop(&mut self) {
        // Best effort: a test that leaves a directory behind has still passed or failed.
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A file store in a directory of its own, and the directory's owner. Bind them in that
/// order: the store drops first, so its lock is released before the directory goes, and
/// both drop when a scenario panics.
fn file_store() -> (TempDir, KeyStore) {
    let dir = crate::test_support::temp_dir("backend-contract");
    std::fs::create_dir_all(&dir).expect("make the directory");
    let store = crate::test_support::open_empty_store(&dir.join("keys.json"));
    (TempDir(dir), store)
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

    /// An id that carries the archive marker is refused where a key is recorded, so no
    /// current key is ever read as an archive: not left out of the list, and not returned
    /// as a predecessor of the key whose id it begins with.
    pub(super) fn a_key_id_cannot_pose_as_an_archive(backend: &dyn KeyBackend) {
        backend.generate_key("issuer").expect("generate");
        backend.rotate_key("issuer").expect("rotate");

        for id in ["issuer#archived-manual", "#archived-", "other#archived-x"] {
            let refused = [
                backend.generate_key(id),
                backend.generate_key_with(id, KeyAlgorithm::EcdsaP256),
                backend.rotate_key(id),
                backend.revoke_and_rotate(id),
            ];
            for result in refused {
                let message = result.err().map(|e| e.to_string()).unwrap_or_default();
                assert!(message.contains("#archived-"), "{id}: {message:?}");
            }
            assert!(!backend.has_key(id), "{id} was not recorded");
        }

        let listed = backend.list().expect("list");
        assert_eq!(listed.len(), 1, "only the current key");
        assert_eq!(listed[0].0, "issuer");
        assert_eq!(
            backend.archived_public_keys("issuer").len(),
            1,
            "only the archive a rotation made"
        );
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

            /// Run `scenario` on a fresh backend. The owner is declared first, so it drops last.
            fn each(scenario: fn(&dyn KeyBackend)) {
                let (_owner, backend) = $make;
                scenario(&backend);
            }

            #[test]
            fn every_algorithm_signs_and_verifies() {
                each(scenarios::every_algorithm_signs_and_verifies);
            }

            #[test]
            fn an_unknown_key_is_not_there() {
                each(scenarios::an_unknown_key_is_not_there);
            }

            #[test]
            fn the_key_loaded_is_the_key_published() {
                each(scenarios::the_key_loaded_is_the_key_published);
            }

            #[test]
            fn an_entry_stays_the_key_it_was_loaded_as() {
                each(scenarios::an_entry_stays_the_key_it_was_loaded_as);
            }

            #[test]
            fn a_rotation_archives_the_old_key_and_installs_a_new_one() {
                each(scenarios::a_rotation_archives_the_old_key_and_installs_a_new_one);
            }

            #[test]
            fn archives_come_oldest_first() {
                each(scenarios::archives_come_oldest_first);
            }

            #[test]
            fn a_rotation_keeps_the_algorithm_unless_told_otherwise() {
                each(scenarios::a_rotation_keeps_the_algorithm_unless_told_otherwise);
            }

            #[test]
            fn a_compromise_archives_the_old_key_revoked() {
                each(scenarios::a_compromise_archives_the_old_key_revoked);
            }

            #[test]
            fn generating_replaces_and_archives_nothing() {
                each(scenarios::generating_replaces_and_archives_nothing);
            }

            #[test]
            fn a_key_id_cannot_pose_as_an_archive() {
                each(scenarios::a_key_id_cannot_pose_as_an_archive);
            }

            #[test]
            fn the_list_names_the_current_keys_by_id() {
                each(scenarios::the_list_names_the_current_keys_by_id);
            }
        }
    };
}

contract_on!(file_backend, file_store());
contract_on!(memory_backend, ((), MemoryKeyStore::new()));

/// A backend that overrides every method the trait provides, and refuses through each.
struct Overriding(MemoryKeyStore);

impl KeyBackend for Overriding {
    fn load_key(&self, key_id: &str) -> Result<KeyEntry> {
        self.0.load_key(key_id)
    }

    fn public_key(&self, key_id: &str) -> Option<PublicKeyInfo> {
        self.0.public_key(key_id)
    }

    fn archived_public_keys(&self, key_id: &str) -> Vec<PublicKeyInfo> {
        self.0.archived_public_keys(key_id)
    }

    fn list(&self) -> Result<Vec<(String, PublicKeyInfo)>> {
        self.0.list()
    }

    fn generate_key_with(&self, key_id: &str, algorithm: KeyAlgorithm) -> Result<KeyEntry> {
        self.0.generate_key_with(key_id, algorithm)
    }

    fn rotate(
        &self,
        key_id: &str,
        revoke_old: bool,
        algorithm: Option<KeyAlgorithm>,
    ) -> Result<KeyEntry> {
        self.0.rotate(key_id, revoke_old, algorithm)
    }

    fn has_key(&self, _key_id: &str) -> bool {
        true
    }

    fn generate_key(&self, _key_id: &str) -> Result<KeyEntry> {
        anyhow::bail!("overridden generate_key")
    }

    fn rotate_key(&self, _key_id: &str) -> Result<KeyEntry> {
        anyhow::bail!("overridden rotate_key")
    }

    fn rotate_key_to(&self, _key_id: &str, _algorithm: KeyAlgorithm) -> Result<KeyEntry> {
        anyhow::bail!("overridden rotate_key_to")
    }

    fn revoke_and_rotate(&self, _key_id: &str) -> Result<KeyEntry> {
        anyhow::bail!("overridden revoke_and_rotate")
    }
}

/// What a backend overrides is reached through an `Arc` as it is directly. The `Arc`
/// forwards every method, the provided ones included, and does not fall back to their
/// defaults.
#[test]
fn an_arc_reaches_what_a_backend_overrides() {
    let direct = Overriding(MemoryKeyStore::new());
    let shared: Arc<dyn KeyBackend> = Arc::new(Overriding(MemoryKeyStore::new()));

    for (how, backend) in [
        ("directly", &direct as &dyn KeyBackend),
        ("in an Arc", &shared),
    ] {
        assert!(backend.has_key("absent"), "{how}: has_key");
        let refused = [
            ("generate_key", backend.generate_key("k")),
            ("rotate_key", backend.rotate_key("k")),
            (
                "rotate_key_to",
                backend.rotate_key_to("k", KeyAlgorithm::Ed25519),
            ),
            ("revoke_and_rotate", backend.revoke_and_rotate("k")),
        ];
        for (name, result) in refused {
            let message = result.err().map(|e| e.to_string());
            assert_eq!(
                message.as_deref(),
                Some(format!("overridden {name}").as_str()),
                "{how}: {name}"
            );
        }
    }
}
