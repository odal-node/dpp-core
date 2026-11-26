//! The algorithm a stored record carries.

use super::super::store::{KeyRecord, default_algorithm};
use super::common::*;

/// The key record's on-disk shape is unchanged by giving `algorithm` a type:
/// it still serialises as the bare JOSE identifier, so an existing store file
/// is byte-compatible.
#[test]
fn algorithm_serialises_as_its_jose_identifier() {
    let record = KeyRecord::new(
        vec![1],
        vec![2],
        "fp".into(),
        "aa".into(),
        crate::jws::KeyAlgorithm::Ed25519,
    );
    let json = serde_json::to_value(&record).expect("serialise");
    assert_eq!(json["algorithm"], "EdDSA");
}

/// A store written before `algorithm` existed still loads, defaulting to the
/// only algorithm there has ever been.
#[test]
fn record_without_algorithm_field_defaults() {
    let record: KeyRecord = serde_json::from_value(serde_json::json!({
        "encrypted_signing_key": [1],
        "nonce": [2],
        "fingerprint": "fp",
        "verifying_key_hex": "aa",
    }))
    .expect("a pre-agility record must still deserialise");
    assert_eq!(record.algorithm, default_algorithm());
}

/// An *unrecognised* algorithm fails closed at deserialisation rather than
/// loading a key that nothing in this crate can safely sign or verify with.
/// Reaching the signing path and failing there would be a worse outcome: the
/// key would look usable right up to the point it was needed.
#[test]
fn record_with_unknown_algorithm_is_refused() {
    let result: Result<KeyRecord, _> = serde_json::from_value(serde_json::json!({
        "encrypted_signing_key": [1],
        "nonce": [2],
        "fingerprint": "fp",
        "verifying_key_hex": "aa",
        "algorithm": "RS256",
    }));
    assert!(
        result.is_err(),
        "a key record naming an algorithm this build does not implement must not load"
    );
}

/// The public half carries its algorithm, so a reader (the `did:web` builder)
/// never has to assume one to choose a JWK shape.
#[test]
fn public_key_info_carries_the_algorithm() {
    let store = temp_store();
    store.generate_key("alg-info").expect("generate");
    let info = store.public_key("alg-info").expect("public key");
    assert_eq!(info.algorithm, default_algorithm());
}

// ─── Legacy-shape refusal, and what each shape costs ─────────────────────────
