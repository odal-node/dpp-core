//! A JWS whose protected header carries `crit` is refused, on both verification
//! paths in this crate (RFC 7515 clause 4.1.11).
//!
//! Every other test here signs with [`signer::sign`], which never writes `crit`,
//! so none of them could notice a verifier that ignores it. These build the
//! token by hand: a *valid* signature over a header that carries `crit`. Each
//! refusal is paired with a control, the same construction without `crit`,
//! which must verify. A refusal can therefore only be the `crit`, never a flaw
//! in the forgery.

use base64::Engine;
use ed25519_dalek::Signer;
use serde_json::{Value, json};

use super::signer;
use super::verifier::verify_jws;
use crate::keystore::KeyStore;
use crate::test_support::temp_store;

const KEY_ID: &str = "crit-key";

/// A compact JWS with a genuine signature over `header` and an arbitrary
/// payload, made with the store's key.
fn forge(store: &KeyStore, header: &Value) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let key = store.load_key(KEY_ID).expect("load key");
    let signing_input = format!(
        "{}.{}",
        b64.encode(serde_json::to_vec(header).expect("header serialises")),
        b64.encode(br#"{"claim":"value"}"#),
    );
    let signature = key.signing_key.sign(signing_input.as_bytes());
    format!("{signing_input}.{}", b64.encode(signature.to_bytes()))
}

/// Whether each verification path accepts `jws`: [`verify_jws`] with the raw
/// public key, and [`signer::verify`] with the key store.
fn accepted(store: &KeyStore, jws: &str) -> [bool; 2] {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let key = store.load_key(KEY_ID).expect("load key");
    let public_key = b64.encode(key.verifying_key.as_bytes());
    [
        verify_jws(jws, &public_key).expect("well-formed input"),
        signer::verify(store, KEY_ID, jws).expect("well-formed input"),
    ]
}

/// The control. If this failed, every refusal below would prove nothing.
#[test]
fn a_forged_jws_without_crit_verifies_on_both_paths() {
    let store = temp_store("crit-control", KEY_ID);
    let jws = forge(&store, &json!({"alg": "EdDSA"}));

    assert_eq!(accepted(&store, &jws), [true, true]);
}

/// A header parameter the recipient does not know is ignored unless `crit`
/// names it, so the refusal is about `crit` and not about extra members.
#[test]
fn an_unlisted_unknown_header_parameter_is_still_accepted() {
    let store = temp_store("crit-unlisted", KEY_ID);
    let jws = forge(&store, &json!({"alg": "EdDSA", "x": 1, "b64": false}));

    assert_eq!(accepted(&store, &jws), [true, true]);
}

/// The case the clause exists for: the producer marked an extension as one the
/// recipient must understand, and this crate understands none.
#[test]
fn crit_naming_an_extension_is_refused_on_both_paths() {
    let store = temp_store("crit-named", KEY_ID);

    for header in [
        json!({"alg": "EdDSA", "crit": ["b64"], "b64": false}),
        json!({"alg": "EdDSA", "crit": ["x"], "x": 1}),
    ] {
        let jws = forge(&store, &header);
        assert_eq!(accepted(&store, &jws), [false, false], "{header}");
    }
}

/// Clause 4.1.11 forbids a producer to emit some shapes and says a recipient
/// may treat them as invalid. Refusing every `crit` costs nothing here, so every
/// shape is refused rather than inspected: an empty list, a registered
/// parameter, a repeated name, a name the header does not carry, and a value
/// that is not a list at all.
#[test]
fn every_shape_of_crit_is_refused_on_both_paths() {
    let store = temp_store("crit-shapes", KEY_ID);

    for crit in [
        json!([]),
        json!(["alg"]),
        json!(["x", "x"]),
        json!(["absent"]),
        json!("b64"),
        json!({}),
        json!(null),
    ] {
        let jws = forge(&store, &json!({"alg": "EdDSA", "x": 1, "crit": crit}));
        assert_eq!(accepted(&store, &jws), [false, false], "crit: {crit}");
    }
}
