//! [`KeyEntry::from_private_jwk`] against the W3C VC-JOSE-COSE test suite's four key
//! files: each builds the key its `kid` names, signs what its `publicKeyJwk` verifies,
//! and takes the store's fingerprint. The refusals are in `private_jwk_refusal_tests`.
//!
//! The key files are the suite's, vendored under `jws/fixtures/w3c-vc-jose-cose/` and
//! read here byte for byte. The helpers in this file are shared with the refusal tests.
//! No private key is written into this file.

use base64::Engine;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use super::KeyEntry;
use super::entry::SigningMaterial;
use super::error::PrivateJwkError;
use crate::jws::algorithm::KeyAlgorithm;
use crate::jws::public_key::PublicKey;
use crate::test_support::{open_empty_store, temp_path};

pub(super) const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// The four key files, in the suite's order, with the algorithm each names.
pub(super) const SUITE: [(&str, KeyAlgorithm); 4] = [
    ("p256", KeyAlgorithm::EcdsaP256),
    ("p384", KeyAlgorithm::EcdsaP384),
    ("p521", KeyAlgorithm::EcdsaP521),
    ("ed25519", KeyAlgorithm::Ed25519),
];

/// The three EC curves of [`SUITE`], each with its algorithm and its group order `n`,
/// big-endian. The order is checked below: `n − 1` is a scalar and `n` is not.
pub(super) const ORDERS: [(&str, KeyAlgorithm, &str); 3] = [
    (
        "p256",
        KeyAlgorithm::EcdsaP256,
        "FFFFFFFF00000000FFFFFFFFFFFFFFFFBCE6FAADA7179E84F3B9CAC2FC632551",
    ),
    (
        "p384",
        KeyAlgorithm::EcdsaP384,
        "FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFC7634D81F4372DDF581A0DB248B0A77AECEC196ACCC52973",
    ),
    (
        "p521",
        KeyAlgorithm::EcdsaP521,
        "01FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFA51868783BF2F966B7FCC0148F709A5D03BB5C9B8899C47AEBB6FB71E91386409",
    ),
];

/// The verification method of one key file: `p256`, `p384`, `p521` or `ed25519`.
fn method(name: &str) -> Value {
    let text = match name {
        "p256" => include_str!("../jws/fixtures/w3c-vc-jose-cose/vm-p256.json"),
        "p384" => include_str!("../jws/fixtures/w3c-vc-jose-cose/vm-p384.json"),
        "p521" => include_str!("../jws/fixtures/w3c-vc-jose-cose/vm-p521.json"),
        "ed25519" => include_str!("../jws/fixtures/w3c-vc-jose-cose/vm-ed25519.json"),
        other => panic!("{other} is not one of the suite's key files"),
    };
    serde_json::from_str(text).expect("a verification method")
}

/// The `secretKeyJwk` of one key file, as a copy the test may change.
pub(super) fn secret(name: &str) -> Value {
    method(name)["secretKeyJwk"].clone()
}

/// The `publicKeyJwk` of one key file.
pub(super) fn public(name: &str) -> Value {
    method(name)["publicKeyJwk"].clone()
}

/// The refusal a JWK gets. A JWK that breaks no rule is a failed test.
pub(super) fn refused(jwk: &Value) -> PrivateJwkError {
    match KeyEntry::from_private_jwk(jwk) {
        Ok(_) => panic!("a JWK that breaks a rule was accepted"),
        Err(error) => error,
    }
}

/// `d` of `jwk`, decoded. Used only to build the test's own copies.
pub(super) fn d_octets(jwk: &Value) -> Vec<u8> {
    B64.decode(jwk["d"].as_str().expect("d"))
        .expect("base64url")
}

/// `jwk` with `d` set to `octets`.
pub(super) fn with_d(mut jwk: Value, octets: &[u8]) -> Value {
    jwk["d"] = json!(B64.encode(octets));
    jwk
}

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
