//! [`KeyEntry::from_private_jwk`] against the W3C VC-JOSE-COSE test suite's four key
//! files: each builds the key its `kid` names, signs what its `publicKeyJwk` verifies,
//! and takes the store's fingerprint. The refusals are in `refusal.rs`.
//!
//! The key files are the suite's, vendored under `jws/fixtures/w3c-vc-jose-cose/` and
//! read here byte for byte. The helpers in this file are shared with the refusal tests.
//! No private key is written into this file.

use serde_json::json;
use sha2::{Digest, Sha256};

use super::super::super::KeyEntry;
use super::super::super::entry::SigningMaterial;
use crate::jws::algorithm::KeyAlgorithm;
use crate::jws::public_key::PublicKey;
use crate::test_support::{open_empty_store, temp_path};

use super::common::{SUITE, public, secret};

// ── The suite's key files ─────────────────────────────────────────────────────

/// Each key file builds the key its curve names, whose thumbprint is the `kid` the
/// file carries, and whose public key is the one its `publicKeyJwk` reads.
#[test]
fn each_suite_key_builds_the_key_its_kid_names() {
    for (name, algorithm) in SUITE {
        let entry = KeyEntry::from_private_jwk(&secret(name)).expect(name);
        let public_jwk = public(name);
        assert_eq!(entry.algorithm(), algorithm, "{name}");
        assert_eq!(
            entry.public_key().thumbprint(),
            public_jwk["kid"].as_str().expect("kid"),
            "{name}"
        );
        assert_eq!(
            entry.public_key(),
            PublicKey::from_jwk(&public_jwk).expect("a key"),
            "{name}"
        );
    }
}

/// A key built from a key file signs a message that the file's `publicKeyJwk`
/// verifies, and does not verify a different message.
#[test]
fn each_suite_key_signs_what_its_public_jwk_verifies() {
    for (name, _) in SUITE {
        let entry = KeyEntry::from_private_jwk(&secret(name)).expect(name);
        let verifier = PublicKey::from_jwk(&public(name)).expect("a key");
        let message = format!("{name}: a signed passport").into_bytes();
        let signature = entry.sign(&message).expect("an unrevoked key signs");
        assert!(verifier.verify(&message, &signature), "{name}");
        assert!(!verifier.verify(b"another message", &signature), "{name}");
    }
}

/// The fingerprint is the hex SHA-256 of the public key, computed here without the
/// crate's helper. A key the store generates carries the same formula.
#[test]
fn the_fingerprint_is_the_store_fingerprint() {
    for (name, _) in SUITE {
        let entry = KeyEntry::from_private_jwk(&secret(name)).expect(name);
        let expected = hex::encode(Sha256::digest(entry.public_key().as_bytes()));
        assert_eq!(entry.fingerprint, expected, "{name}");
        assert!(!entry.revoked, "{name}");
    }

    let path = temp_path("private-jwk-fingerprint");
    let store = open_empty_store(&path);
    let (record, entry) = store
        .seal_new_record(SigningMaterial::generate(KeyAlgorithm::EcdsaP384))
        .expect("seal");
    let expected = hex::encode(Sha256::digest(entry.public_key().as_bytes()));
    assert_eq!(entry.fingerprint, expected, "the store's own key");
    assert_eq!(record.fingerprint, expected, "the record sealed for it");
    assert!(!path.exists(), "sealing writes nothing to disk");
}

/// `ed25519` declared as `Ed25519` (RFC 9864's name for the same operation) builds.
#[test]
fn ed25519_declared_as_ed25519_builds() {
    let mut jwk = secret("ed25519");
    jwk["alg"] = json!("Ed25519");
    let entry = KeyEntry::from_private_jwk(&jwk).expect("RFC 9864's name");
    assert_eq!(entry.algorithm(), KeyAlgorithm::Ed25519);
}

/// `alg` is optional: a key without one builds the same key.
#[test]
fn an_absent_alg_is_not_refused() {
    for (name, algorithm) in SUITE {
        let mut jwk = secret(name);
        jwk.as_object_mut().expect("an object").remove("alg");
        assert_eq!(
            KeyEntry::from_private_jwk(&jwk).expect(name).algorithm(),
            algorithm,
            "{name}"
        );
    }
}

/// `kid`, `use` and `key_ops` are not read, so a key with other values still builds.
#[test]
fn kid_use_and_key_ops_are_not_read() {
    for (name, algorithm) in SUITE {
        let mut jwk = secret(name);
        jwk["kid"] = json!("not-the-thumbprint");
        jwk["use"] = json!("enc");
        jwk["key_ops"] = json!(["encrypt"]);
        assert_eq!(
            KeyEntry::from_private_jwk(&jwk).expect(name).algorithm(),
            algorithm,
            "{name}"
        );
    }
}
