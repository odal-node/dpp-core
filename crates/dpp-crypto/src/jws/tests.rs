use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::keystore::KeyStore;
use crate::test_support::temp_store;

// ── signer tests ─────────────────────────────────────────────────────────────

#[test]
fn sign_then_verify_roundtrip() {
    let store = temp_store("signer", "t1");
    let payload = json!({"passport_id": "abc-123", "status": "draft"});
    let jws = super::signer::sign(&store, "t1", &payload).expect("sign");
    assert!(
        super::signer::verify(&store, "t1", &jws).expect("verify"),
        "signature must be valid"
    );
}

#[test]
fn tampered_signature_fails_verification() {
    let store = temp_store("signer-tamper", "t2");
    let payload = json!({"data": "hello"});
    let mut jws = super::signer::sign(&store, "t2", &payload).expect("sign");

    let last = jws.pop().unwrap();
    jws.push(if last == 'A' { 'B' } else { 'A' });

    let ok = super::signer::verify(&store, "t2", &jws).unwrap_or(false);
    assert!(!ok, "tampered JWS must fail verification");
}

#[test]
fn jws_has_three_parts() {
    let store = temp_store("signer-parts", "t3");
    let payload = json!({"x": 1});
    let jws = super::signer::sign(&store, "t3", &payload).expect("sign");
    assert_eq!(
        jws.splitn(4, '.').count(),
        3,
        "JWS must have exactly 3 dot-separated parts"
    );
}

#[test]
fn key_material_absent_from_jws() {
    let store = temp_store("signer-keymaterial", "t4");
    let loaded = store.load_key("t4").expect("load");
    let payload = json!({"test": true});
    let jws = super::signer::sign(&store, "t4", &payload).expect("sign");

    let signing_key_hex = hex::encode(loaded.signing_key.as_bytes());
    assert!(
        !jws.contains(&signing_key_hex),
        "JWS must not contain raw private key bytes"
    );
}

#[test]
#[tracing_test::traced_test]
fn key_material_absent_from_logs() {
    let store = temp_store("signer-logs", "t5");
    let loaded = store.load_key("t5").expect("load");
    let priv_hex = hex::encode(loaded.signing_key.as_bytes());
    let payload = json!({"passport_id": "p-log-safety", "status": "draft"});
    let _ = super::signer::sign(&store, "t5", &payload).expect("sign");
    assert!(
        !logs_contain(&priv_hex),
        "private key material must never appear in any log line"
    );
}

/// Regression (L-4): the keystore verify path also pins `alg` to EdDSA, so a
/// header claiming a different algorithm is rejected even when the signature
/// over the signing input is otherwise valid.
#[test]
fn signer_verify_rejects_non_eddsa_alg() {
    let store = temp_store("signer-alg", "t6");
    let loaded = store.load_key("t6").expect("load");
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let payload_b64 = b64.encode(serde_json::to_vec(&json!({"id": "x"})).unwrap());
    for alg in ["none", "HS256", "RS256"] {
        let header_b64 = b64.encode(format!(r#"{{"alg":"{alg}","kid":"x"}}"#));
        let signing_input = format!("{header_b64}.{payload_b64}");
        let sig_b64 = b64.encode(loaded.signing_key.sign(signing_input.as_bytes()).to_bytes());
        let jws = format!("{signing_input}.{sig_b64}");
        assert!(
            !super::signer::verify(&store, "t6", &jws).unwrap(),
            "keystore verify must reject alg={alg}"
        );
    }
}

// ── verifier tests ────────────────────────────────────────────────────────────

fn make_jws(signing_key: &SigningKey, payload: &serde_json::Value) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let header = r#"{"alg":"EdDSA","crv":"Ed25519"}"#;
    let header_b64 = b64.encode(header);
    let payload_bytes = serde_json::to_vec(payload).unwrap();
    let payload_b64 = b64.encode(&payload_bytes);
    let signing_input = format!("{header_b64}.{payload_b64}");
    let sig = signing_key.sign(signing_input.as_bytes());
    let sig_b64 = b64.encode(sig.to_bytes());
    format!("{signing_input}.{sig_b64}")
}

fn pub_key_b64(signing_key: &SigningKey) -> String {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    b64.encode(signing_key.verifying_key().as_bytes())
}

#[test]
fn valid_jws_verifies() {
    let key = SigningKey::generate(&mut crate::os_rng());
    let payload = json!({"id": "test", "status": "published"});
    let jws = make_jws(&key, &payload);
    assert!(super::verifier::verify_jws(&jws, &pub_key_b64(&key)).unwrap());
}

#[test]
fn tampered_signature_fails() {
    let key = SigningKey::generate(&mut crate::os_rng());
    let payload = json!({"id": "test"});
    let mut jws = make_jws(&key, &payload);
    let last = jws.pop().unwrap();
    jws.push(if last == 'A' { 'B' } else { 'A' });
    assert!(
        matches!(
            super::verifier::verify_jws(&jws, &pub_key_b64(&key)),
            Ok(false) | Err(_)
        ),
        "tampered JWS must be rejected"
    );
}

#[test]
fn wrong_key_fails() {
    let key1 = SigningKey::generate(&mut crate::os_rng());
    let key2 = SigningKey::generate(&mut crate::os_rng());
    let payload = json!({"id": "test"});
    let jws = make_jws(&key1, &payload);
    assert!(!super::verifier::verify_jws(&jws, &pub_key_b64(&key2)).unwrap());
}

/// Regression (red-team ATK-4): the header `alg` is pinned to EdDSA.
#[test]
fn non_eddsa_alg_is_rejected() {
    let key = SigningKey::generate(&mut crate::os_rng());
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let payload_b64 = b64.encode(serde_json::to_vec(&json!({"id": "x"})).unwrap());
    for alg in ["none", "HS256", "RS256", "ES256"] {
        let header_b64 = b64.encode(format!(r#"{{"alg":"{alg}"}}"#));
        let signing_input = format!("{header_b64}.{payload_b64}");
        let sig_b64 = b64.encode(key.sign(signing_input.as_bytes()).to_bytes());
        let jws = format!("{signing_input}.{sig_b64}");
        assert!(
            !super::verifier::verify_jws(&jws, &pub_key_b64(&key)).unwrap(),
            "alg={alg} must be rejected"
        );
    }
}

#[test]
fn extract_key_from_did_document() {
    let doc = json!({
        "verificationMethod": [{
            "id": "did:web:example.com#key-1",
            "type": "JsonWebKey2020",
            "controller": "did:web:example.com",
            "publicKeyJwk": { "kty": "OKP", "crv": "Ed25519", "x": "abc123" }
        }],
        "assertionMethod": ["did:web:example.com#key-1"]
    });
    assert_eq!(
        super::verifier::extract_primary_public_key(&doc),
        Some("abc123".to_string())
    );
}

/// crypto Gap 3: only `kty:"OKP", crv:"Ed25519"` JWKs are accepted.
#[test]
fn non_ed25519_jwk_is_rejected() {
    let doc = |jwk| {
        json!({
            "verificationMethod": [{ "id": "did:x#k1", "publicKeyJwk": jwk }],
            "assertionMethod": ["did:x#k1"]
        })
    };
    let wrong_curve = doc(json!({"kty":"OKP","crv":"X25519","x":"a"}));
    let wrong_type = doc(json!({"kty":"EC","crv":"P-256","x":"a"}));
    let untyped = doc(json!({"x":"a"}));
    assert!(super::verifier::extract_primary_public_key(&wrong_curve).is_none());
    assert!(super::verifier::extract_primary_public_key(&wrong_type).is_none());
    assert!(super::verifier::extract_primary_public_key(&untyped).is_none());
}

/// crypto Gap 3 (verification relationship): key not in assertionMethod is rejected.
#[test]
fn key_not_in_assertion_method_is_rejected() {
    let doc = json!({
        "verificationMethod": [{
            "id": "did:web:example.com#key-1",
            "publicKeyJwk": { "kty": "OKP", "crv": "Ed25519", "x": "abc123" }
        }],
        "authentication": ["did:web:example.com#key-1"],
        "assertionMethod": []
    });
    assert!(
        super::verifier::extract_primary_public_key(&doc).is_none(),
        "a key not in assertionMethod must not be selected as a signer"
    );
    let no_am = json!({
        "verificationMethod": [{
            "id": "did:web:example.com#key-1",
            "publicKeyJwk": { "kty": "OKP", "crv": "Ed25519", "x": "abc123" }
        }]
    });
    assert!(super::verifier::extract_primary_public_key(&no_am).is_none());
}

#[test]
fn extract_kid_returns_kid_when_present() {
    let key = SigningKey::generate(&mut crate::os_rng());
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let payload = json!({"test": 1});
    let payload_b64 = b64.encode(serde_json::to_vec(&payload).unwrap());
    let header = r#"{"alg":"EdDSA","crv":"Ed25519","kid":"deadbeef1234"}"#;
    let header_b64 = b64.encode(header);
    let signing_input = format!("{header_b64}.{payload_b64}");
    let sig_b64 = b64.encode(key.sign(signing_input.as_bytes()).to_bytes());
    let jws = format!("{signing_input}.{sig_b64}");
    assert_eq!(
        super::verifier::extract_kid_from_jws(&jws),
        Some("deadbeef1234".to_string())
    );
}

#[test]
fn extract_kid_returns_none_when_absent() {
    let key = SigningKey::generate(&mut crate::os_rng());
    let payload = json!({"test": 1});
    let jws = make_jws(&key, &payload);
    assert_eq!(super::verifier::extract_kid_from_jws(&jws), None);
}

#[test]
fn extract_key_by_fingerprint_finds_archived_key() {
    let key1 = SigningKey::generate(&mut crate::os_rng());
    let key2 = SigningKey::generate(&mut crate::os_rng());

    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let key1_x = b64.encode(key1.verifying_key().as_bytes());
    let key2_x = b64.encode(key2.verifying_key().as_bytes());

    let did_doc = json!({
        "verificationMethod": [
            {"id": "did:x#key-1", "publicKeyJwk": {"kty": "OKP", "crv": "Ed25519", "x": key2_x}},
            {"id": "did:x#key-2", "publicKeyJwk": {"kty": "OKP", "crv": "Ed25519", "x": key1_x}}
        ],
        "assertionMethod": ["did:x#key-1", "did:x#key-2"]
    });

    let key1_fp = hex::encode(Sha256::digest(key1.verifying_key().as_bytes()));
    let found = super::verifier::extract_key_by_fingerprint(&did_doc, &key1_fp);
    assert_eq!(
        found,
        Some(key1_x),
        "must find the archived key by fingerprint"
    );

    let found_wrong =
        super::verifier::extract_key_by_fingerprint(&did_doc, "nonexistentfingerprint");
    assert!(found_wrong.is_none(), "unknown fingerprint returns None");
}

/// Regression (W-2): sign with key A, rotate to key B, verify old JWS against
/// a DID document that contains both keys.
#[test]
fn rotation_does_not_break_old_jws_verification() {
    let path = std::env::temp_dir().join(format!("test-w2-rotation-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(&path, "rotation-test").expect("open store");
    store.generate_key("issuer").expect("generate key A");

    let payload = json!({"product": "battery", "status": "draft"});
    let jws_a = super::signer::sign(&store, "issuer", &payload).expect("sign with A");

    store.archive_key("issuer").expect("archive A");
    store.generate_key("issuer").expect("generate key B");

    let did_doc = crate::identity::did_builder::build_did_document(
        &store,
        "https://id.example.com",
        "issuer",
    )
    .expect("build DID doc");

    assert_eq!(
        did_doc["verificationMethod"].as_array().unwrap().len(),
        2,
        "DID doc must list both keys after rotation"
    );

    let kid = super::verifier::extract_kid_from_jws(&jws_a).expect("kid must be present");
    let pub_key = super::verifier::extract_key_by_fingerprint(&did_doc, &kid)
        .expect("archived key must be found by fingerprint");

    let ok = super::verifier::verify_jws(&jws_a, &pub_key).expect("verify must not error");
    assert!(
        ok,
        "old JWS must verify against archived key after rotation"
    );

    let jws_b = super::signer::sign(&store, "issuer", &payload).expect("sign with B");
    let kid_b = super::verifier::extract_kid_from_jws(&jws_b).expect("kid must be present");
    let pub_key_b = super::verifier::extract_key_by_fingerprint(&did_doc, &kid_b)
        .expect("current key must be found by fingerprint");
    let ok_b = super::verifier::verify_jws(&jws_b, &pub_key_b).expect("verify must not error");
    assert!(ok_b, "new JWS must verify against current key");
}

// ── G-4: Cross-library JWS golden vector ──────────────────────────────────────

/// G-4: Cross-library JWS golden vector.
///
/// Signs a payload with our signer (JCS-canonical bytes) and independently
/// verifies with raw ed25519_dalek — bypassing our verifier module entirely.
/// Also pins that the JWS payload part is exactly the JCS canonical bytes, not
/// incidental serde output (proves canonicalization is applied before signing).
#[test]
fn jws_golden_vector_raw_dalek_verification() {
    let store = temp_store("g4", "g4-key");
    let loaded = store.load_key("g4-key").expect("load key");
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    // Out-of-order keys — JCS must sort them: {"a":1,"nested":{"x":"a","y":"b"},"z":2}
    let payload = json!({"z": 2, "a": 1, "nested": {"y": "b", "x": "a"}});
    let jws = super::signer::sign(&store, "g4-key", &payload).expect("sign");

    let parts: Vec<&str> = jws.splitn(3, '.').collect();
    assert_eq!(parts.len(), 3, "JWS must have 3 parts");

    // Pin: payload part must be the JCS-canonical bytes (object keys sorted by UTF-16 code unit).
    let jws_payload_bytes = b64.decode(parts[1]).expect("decode payload b64");
    let expected_canonical = super::canonical::canonicalize(&payload).expect("canonicalize");
    assert_eq!(
        jws_payload_bytes, expected_canonical,
        "JWS payload must be JCS canonical bytes; signer must not use incidental serde output"
    );

    // Cross-library verification: use raw ed25519_dalek::verify_strict,
    // independent of our verifier module, to confirm the signing format is correct.
    let sig_bytes = b64.decode(parts[2]).expect("decode sig b64");
    let sig_arr: [u8; 64] = sig_bytes.try_into().expect("Ed25519 signature is 64 bytes");
    let raw_sig = ed25519_dalek::Signature::from_bytes(&sig_arr);
    let signing_input = format!("{}.{}", parts[0], parts[1]);
    assert!(
        loaded
            .verifying_key
            .verify_strict(signing_input.as_bytes(), &raw_sig)
            .is_ok(),
        "raw ed25519_dalek::verify_strict must accept the JWS produced by our signer"
    );
}

/// G-4 (true independent cross-check): the test above
/// (`jws_golden_vector_raw_dalek_verification`) uses raw `ed25519_dalek`,
/// which is the **same** library the production signer/verifier already
/// depend on — it bypasses our `verifier` module's logic, but it does not
/// prove a genuinely different implementation accepts the signature. `ring`
/// is a separate Ed25519 codebase (not built on `curve25519-dalek`), added
/// as a dev-only dependency purely for this check. This is the strongest
/// "an off-the-shelf JOSE-capable library can verify this" proof available
/// without leaving the Rust/cargo toolchain.
#[test]
fn jws_golden_vector_independent_ring_verification() {
    let store = temp_store("g4-ring", "g4-ring-key");
    let loaded = store.load_key("g4-ring-key").expect("load key");
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let payload = json!({"z": 2, "a": 1, "nested": {"y": "b", "x": "a"}});
    let jws = super::signer::sign(&store, "g4-ring-key", &payload).expect("sign");

    let parts: Vec<&str> = jws.splitn(3, '.').collect();
    assert_eq!(parts.len(), 3, "JWS must have 3 parts");

    let signing_input = format!("{}.{}", parts[0], parts[1]);
    let sig_bytes = b64.decode(parts[2]).expect("decode sig b64");
    let pub_key_bytes = loaded.verifying_key.as_bytes();

    let ring_key =
        ring::signature::UnparsedPublicKey::new(&ring::signature::ED25519, pub_key_bytes);
    ring_key
        .verify(signing_input.as_bytes(), &sig_bytes)
        .expect(
            "an independent Ed25519 implementation (ring) must accept the \
             JWS our signer produced — proves the wire format is genuinely \
             standard EdDSA, not an artefact of ed25519-dalek's own encoding",
        );

    // And the inverse: ring must reject a tampered signature, same as our verifier.
    let mut tampered_sig = sig_bytes.clone();
    tampered_sig[0] ^= 0xFF;
    assert!(
        ring_key
            .verify(signing_input.as_bytes(), &tampered_sig)
            .is_err(),
        "ring must reject a tampered signature"
    );
}

/// Gap 7 end-to-end: after a key is **revoked**, a JWS it produced must no
/// longer be verifiable — the revoked key is absent from the DID document.
#[test]
fn revoked_key_signature_no_longer_verifies() {
    let path =
        std::env::temp_dir().join(format!("test-revoke-verify-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(&path, "rev-verify").expect("open store");
    store.generate_key("issuer").expect("generate key A");

    let payload = json!({"product": "battery", "status": "draft"});
    let jws_a = super::signer::sign(&store, "issuer", &payload).expect("sign with A");

    store.revoke_and_rotate("issuer").expect("revoke+rotate");
    let did_doc = crate::identity::did_builder::build_did_document(
        &store,
        "https://id.example.com",
        "issuer",
    )
    .expect("build DID doc");

    let kid = super::verifier::extract_kid_from_jws(&jws_a).expect("kid present");
    assert!(
        super::verifier::extract_key_by_fingerprint(&did_doc, &kid).is_none(),
        "a revoked key must not be selectable for verification"
    );

    let jws_b = super::signer::sign(&store, "issuer", &payload).expect("sign with B");
    let kid_b = super::verifier::extract_kid_from_jws(&jws_b).expect("kid present");
    let pub_b = super::verifier::extract_key_by_fingerprint(&did_doc, &kid_b)
        .expect("current key resolves");
    assert!(super::verifier::verify_jws(&jws_b, &pub_b).expect("verify"));
}
