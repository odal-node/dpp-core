//! Unit tests for the vendored JWS verifier.

use super::*;
use base64::Engine;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};

fn sign_compact(
    signing_key: &SigningKey,
    header: &serde_json::Value,
    payload: &serde_json::Value,
) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header_b64 = b64.encode(serde_json::to_vec(header).unwrap());
    let payload_b64 = b64.encode(serde_json::to_vec(payload).unwrap());
    let signing_input = format!("{header_b64}.{payload_b64}");
    let sig = signing_key.sign(signing_input.as_bytes());
    format!("{signing_input}.{}", b64.encode(sig.to_bytes()))
}

fn did_doc_for(verifying_key: &VerifyingKey) -> serde_json::Value {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let x = b64.encode(verifying_key.to_bytes());
    serde_json::json!({
        "verificationMethod": [{
            "id": "did:web:example.com#root",
            "type": "JsonWebKey2020",
            "publicKeyJwk": { "kty": "OKP", "crv": "Ed25519", "x": x },
        }],
        "assertionMethod": ["did:web:example.com#root"],
    })
}

#[test]
fn valid_signature_verifies() {
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let header = serde_json::json!({"alg": "EdDSA", "typ": "JWT"});
    let payload = serde_json::json!({"hello": "world"});
    let jws = sign_compact(&signing_key, &header, &payload);
    let did_doc = did_doc_for(&signing_key.verifying_key());
    let key = extract_primary_public_key(&did_doc).unwrap();
    assert!(verify_jws(&jws, &key).unwrap());
}

#[test]
fn tampered_payload_fails() {
    let signing_key = SigningKey::from_bytes(&[7u8; 32]);
    let header = serde_json::json!({"alg": "EdDSA", "typ": "JWT"});
    let payload = serde_json::json!({"hello": "world"});
    let jws = sign_compact(&signing_key, &header, &payload);
    let did_doc = did_doc_for(&signing_key.verifying_key());
    let key = extract_primary_public_key(&did_doc).unwrap();

    // Flip one byte in the payload segment.
    let parts: Vec<&str> = jws.splitn(3, '.').collect();
    let tampered = format!("{}.{}x.{}", parts[0], parts[1], parts[2]);
    assert!(!verify_jws(&tampered, &key).unwrap());
}

#[test]
fn alg_none_is_rejected() {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = b64.encode(serde_json::to_vec(&serde_json::json!({"alg": "none"})).unwrap());
    let payload = b64.encode(serde_json::to_vec(&serde_json::json!({"a": 1})).unwrap());
    let fake_jws = format!("{header}.{payload}.");
    assert!(!verify_jws(&fake_jws, "AAAA").unwrap());
}
