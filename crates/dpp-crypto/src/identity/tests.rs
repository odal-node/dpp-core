use std::sync::Arc;

use base64::Engine;
use serde_json::json;

use sha2::Digest;

use super::did_builder::build_did_document;
use super::local_service::LocalIdentityService;
use crate::keystore::KeyStore;

use dpp_domain::{PassportId, ports::identity_port::IdentityPort};

fn temp_store(label: &str, key_id: &str) -> KeyStore {
    let path = std::env::temp_dir().join(format!("test-{label}-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(path, "test-pass").expect("open store");
    store.generate_key(key_id).expect("generate key");
    store
}

fn test_service() -> LocalIdentityService {
    let path = std::env::temp_dir().join(format!("test-identity-{}.json", uuid::Uuid::now_v7()));
    let store = KeyStore::open(&path, "test-passphrase").expect("open store");
    store.generate_key("test-issuer").expect("generate key");
    LocalIdentityService::new(
        Arc::new(store),
        "test-issuer".into(),
        "https://id.example.com".into(),
    )
}

// ── did_builder tests ─────────────────────────────────────────────────────────

#[test]
fn did_document_has_required_fields() {
    let store = temp_store("did-fields", "acme");
    let doc = build_did_document(&store, "https://example.com", "acme").expect("build");

    assert!(doc["@context"].is_array(), "@context must be array");
    assert!(doc["id"].is_string(), "id must be string");
    assert!(
        doc["verificationMethod"].is_array(),
        "verificationMethod must be array"
    );
    assert!(
        doc["assertionMethod"].is_array(),
        "assertionMethod must be array"
    );
    assert!(
        doc["authentication"].is_array(),
        "authentication must be array"
    );
}

#[test]
fn did_id_is_pathless_did_web() {
    let store = temp_store("did-id", "widgets-inc");
    let doc =
        build_did_document(&store, "https://identity.odal-node.io", "widgets-inc").expect("build");
    let id = doc["id"].as_str().expect("id is string");
    assert_eq!(id, "did:web:identity.odal-node.io");
}

#[test]
fn primary_key_is_key_1() {
    let store = temp_store("did-primary", "t-primary");
    let doc = build_did_document(&store, "https://id.example.com", "t-primary").expect("build");
    let vms = doc["verificationMethod"].as_array().expect("array");
    let first_id = vms[0]["id"].as_str().expect("string");
    assert!(
        first_id.ends_with("#key-1"),
        "first verification method must be #key-1"
    );
}

#[test]
fn archived_keys_appear_as_secondary_methods_after_rotation() {
    let store = temp_store("did-rotate", "t-rotate");
    store.archive_key("t-rotate").expect("archive 1");
    store.generate_key("t-rotate").expect("new key 1");
    store.archive_key("t-rotate").expect("archive 2");
    store.generate_key("t-rotate").expect("new key 2");

    let doc = build_did_document(&store, "https://id.example.com", "t-rotate").expect("build");
    let vms = doc["verificationMethod"].as_array().expect("array");
    assert_eq!(
        vms.len(),
        3,
        "should have 3 verification methods after 2 rotations"
    );
    assert!(vms[1]["id"].as_str().unwrap().ends_with("#key-2"));
    assert!(vms[2]["id"].as_str().unwrap().ends_with("#key-3"));
}

/// Gap 7: a revoked key must not appear in the DID document at all.
#[test]
fn revoked_key_is_excluded_from_did_document() {
    let store = temp_store("did-revoke", "t-rev");

    // Hygiene rotation: old key archived but still valid → 2 methods.
    store.rotate_key("t-rev").expect("rotate");
    let doc = build_did_document(&store, "https://id.example.com", "t-rev").expect("build");
    assert_eq!(
        doc["verificationMethod"].as_array().unwrap().len(),
        2,
        "hygiene-archived key remains published"
    );

    // Compromise rotation: previous current key archived + revoked.
    store.revoke_and_rotate("t-rev").expect("revoke+rotate");
    let doc = build_did_document(&store, "https://id.example.com", "t-rev").expect("build");
    let vms = doc["verificationMethod"].as_array().unwrap();
    let am = doc["assertionMethod"].as_array().unwrap();
    assert_eq!(vms.len(), 2, "revoked key must be excluded, got {vms:?}");
    assert_eq!(am.len(), 2, "revoked key must not be an assertionMethod");
}

/// C-1 regression: VM type and @context must agree.
/// `JsonWebKey2020` (publicKeyJwk) requires the jws-2020 context, NOT
/// ed25519-2020 (which pairs with Ed25519VerificationKey2020 + publicKeyMultibase).
#[test]
fn did_document_context_and_vm_type_are_consistent() {
    let store = temp_store("did-ctx", "t-ctx");
    let doc = build_did_document(&store, "https://id.example.com", "t-ctx").expect("build");

    let contexts: Vec<&str> = doc["@context"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert!(
        contexts.contains(&"https://w3id.org/security/suites/jws-2020/v1"),
        "JsonWebKey2020 VMs require the jws-2020 context; got: {contexts:?}"
    );
    assert!(
        !contexts.contains(&"https://w3id.org/security/suites/ed25519-2020/v1"),
        "ed25519-2020 context is for Ed25519VerificationKey2020, not JsonWebKey2020"
    );

    let vms = doc["verificationMethod"].as_array().unwrap();
    for vm in vms {
        assert_eq!(
            vm["type"].as_str().unwrap(),
            "JsonWebKey2020",
            "all VMs must be JsonWebKey2020 to match the declared context"
        );
        assert!(
            vm.get("publicKeyJwk").is_some(),
            "JsonWebKey2020 VM must carry publicKeyJwk"
        );
        assert!(
            vm.get("publicKeyMultibase").is_none(),
            "publicKeyMultibase belongs to Ed25519VerificationKey2020, not JsonWebKey2020"
        );
    }
}

/// G-6: DID document golden vector — key material correctly encoded as JWK.
///
/// Pins that `verificationMethod[0].publicKeyJwk` has the correct JWK shape and
/// that `x` is exactly the base64url-encoded raw 32-byte verifying key from the
/// keystore. This is the cross-check that the DID builder does not corrupt or
/// mis-encode key material.
#[test]
fn did_document_jwk_x_matches_keystore_public_key() {
    let store = temp_store("did-g6", "g6-key");
    let loaded = store.load_key("g6-key").expect("load key");
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;

    let doc = build_did_document(&store, "https://id.example.com", "g6-key").expect("build");
    let vms = doc["verificationMethod"].as_array().unwrap();
    let primary_jwk = &vms[0]["publicKeyJwk"];

    assert_eq!(
        primary_jwk["kty"].as_str().unwrap(),
        "OKP",
        "kty must be OKP (RFC 8037)"
    );
    assert_eq!(
        primary_jwk["crv"].as_str().unwrap(),
        "Ed25519",
        "crv must be Ed25519"
    );

    // Pin: x must be base64url(verifying_key_bytes), not re-encoded or truncated.
    let expected_x = b64.encode(loaded.verifying_key.as_bytes());
    assert_eq!(
        primary_jwk["x"].as_str().unwrap(),
        expected_x,
        "publicKeyJwk.x must equal base64url(verifying_key.as_bytes()) from keystore"
    );

    // No private key material in JWK.
    assert!(
        primary_jwk.get("d").is_none(),
        "private key parameter 'd' must never appear in a published JWK"
    );
}

/// G-6: full DID Core structural validity on a realistic keystore (primary +
/// one hygiene-archived key + one revoked-and-rotated key), the exact
/// scenario the release review asked for. Goes beyond the narrower
/// `did_document_jwk_x_matches_keystore_public_key` (key-material shape) and
/// `did_document_context_and_vm_type_are_consistent` (C-1) checks above by
/// validating the document as a whole, including **referential integrity**
/// between `authentication`/`assertionMethod` and `verificationMethod` —
/// a real DID Core requirement that nothing else here checks: an
/// `authentication`/`assertionMethod` entry that doesn't resolve to an
/// `id` actually present in `verificationMethod` is a malformed DID
/// document, full stop, independent of any JSON-LD context resolution.
#[test]
fn did_document_is_structurally_valid_did_core_after_rotation_and_revocation() {
    let store = temp_store("did-structural", "g6-rotate");

    // Hygiene rotation: old key archived but still valid.
    store.rotate_key("g6-rotate").expect("hygiene rotate");
    // Compromise rotation: now-previous key archived + revoked.
    store.revoke_and_rotate("g6-rotate").expect("revoke+rotate");

    let doc = build_did_document(&store, "https://id.example.com", "g6-rotate").expect("build");

    // `id` is a well-formed pathless did:web identifier.
    let id = doc["id"].as_str().expect("id must be a string");
    assert!(
        id.starts_with("did:web:") && !id.contains('/'),
        "id must be a pathless did:web DID, got {id}"
    );

    // `@context` is a non-empty array of absolute https URIs.
    let contexts = doc["@context"].as_array().expect("@context must be array");
    assert!(!contexts.is_empty(), "@context must not be empty");
    for c in contexts {
        let s = c.as_str().expect("@context entries must be strings");
        assert!(
            s.starts_with("https://"),
            "@context entry must be an absolute https URI, got {s}"
        );
    }

    // `verificationMethod` entries are individually well-formed and scoped
    // to this DID (revoked key already excluded — covered elsewhere, but
    // re-asserted here as part of "the whole document is valid").
    let vms = doc["verificationMethod"]
        .as_array()
        .expect("verificationMethod must be array");
    assert_eq!(vms.len(), 2, "revoked key excluded, hygiene-archived kept");
    let mut seen_ids = std::collections::HashSet::new();
    for vm in vms {
        let vm_id = vm["id"].as_str().expect("VM id must be a string");
        assert!(
            vm_id.starts_with(&format!("{id}#")),
            "VM id must be a fragment of the document id, got {vm_id}"
        );
        assert!(seen_ids.insert(vm_id), "duplicate VM id: {vm_id}");
        assert_eq!(
            vm["controller"].as_str().unwrap(),
            id,
            "controller must equal the document id"
        );
        assert!(vm["type"].is_string(), "VM must declare a type");
        assert!(
            vm["publicKeyJwk"].is_object(),
            "VM must carry publicKeyJwk key material"
        );
    }

    // Referential integrity: every authentication/assertionMethod entry
    // must resolve to a verificationMethod actually present in the document.
    for rel in ["authentication", "assertionMethod"] {
        let refs = doc[rel]
            .as_array()
            .unwrap_or_else(|| panic!("{rel} must be an array"));
        assert!(!refs.is_empty(), "{rel} must not be empty");
        for r in refs {
            let r_id = r
                .as_str()
                .unwrap_or_else(|| panic!("{rel} entries must be strings"));
            assert!(
                seen_ids.contains(r_id),
                "{rel} entry '{r_id}' does not resolve to any verificationMethod id \
                 (dangling reference — a malformed DID document)"
            );
        }
    }
}

// ── local_service tests ───────────────────────────────────────────────────────

#[tokio::test]
async fn sign_and_verify_round_trip() {
    let svc = test_service();
    let payload = json!({"product": "widget", "status": "draft"});
    let credential = svc
        .sign_passport(PassportId::new(), &payload)
        .await
        .expect("sign");

    assert!(!credential.jws.is_empty());
    assert!(credential.issuer_did.contains("id.example.com"));

    let valid = svc
        .verify_signature(&credential.jws, &payload)
        .await
        .expect("verify");
    assert!(valid);
}

/// Content-binding (crypto Gap 8): a valid JWS signed over payload A must
/// NOT verify when presented alongside a different payload B.
#[tokio::test]
async fn signature_is_bound_to_its_payload() {
    let svc = test_service();
    let payload_a = json!({"product": "widget", "status": "draft", "co2e": 1.5});
    let credential = svc
        .sign_passport(PassportId::new(), &payload_a)
        .await
        .expect("sign");

    let payload_b = json!({"product": "widget", "status": "draft", "co2e": 9.9});
    let bound = svc
        .verify_signature(&credential.jws, &payload_b)
        .await
        .expect("verify");
    assert!(
        !bound,
        "JWS for payload A must not verify against payload B"
    );

    assert!(
        svc.verify_signature(&credential.jws, &payload_a)
            .await
            .expect("verify"),
        "JWS must verify against the payload it was signed over"
    );
}

/// Content-binding must be robust to re-serialization: canonically equal
/// payloads with reordered keys / integer-valued floats still verify.
#[tokio::test]
async fn content_binding_is_canonical_not_byte_incidental() {
    let svc = test_service();
    let signed = json!({"b": 2.0, "a": 1, "nested": {"y": 1, "x": 2}});
    let credential = svc
        .sign_passport(PassportId::new(), &signed)
        .await
        .expect("sign");

    let reordered = json!({"nested": {"x": 2, "y": 1}, "a": 1, "b": 2});
    assert!(
        svc.verify_signature(&credential.jws, &reordered)
            .await
            .expect("verify"),
        "canonically-equal payload must verify regardless of key order / number form"
    );
}

#[tokio::test]
async fn tampered_jws_fails_verification() {
    let svc = test_service();
    let payload = json!({"data": "test"});
    let credential = svc
        .sign_passport(PassportId::new(), &payload)
        .await
        .expect("sign");

    let mut tampered = credential.jws.clone();
    let last = tampered.pop().unwrap();
    tampered.push(if last == 'A' { 'B' } else { 'A' });

    let valid = svc.verify_signature(&tampered, &payload).await;
    assert!(matches!(valid, Ok(false) | Err(_)));
}

/// Gap 10: `SignedCredential.credential` must be a proper W3C VC 2.0 envelope.
#[tokio::test]
async fn sign_passport_credential_is_typed_vc() {
    let svc = test_service();
    let payload = json!({"co2e_kg": 1.5, "material": "aluminium"});
    let signed = svc
        .sign_passport(PassportId::new(), &payload)
        .await
        .expect("sign");

    let vc = &signed.credential;
    assert_eq!(
        vc.context[0], "https://www.w3.org/ns/credentials/v2",
        "must include W3C VC 2.0 context"
    );
    assert!(
        vc.credential_type
            .iter()
            .any(|t| t == "DppPassportCredential"),
        "type must include DppPassportCredential"
    );
    assert!(
        vc.issuer.starts_with("did:web:"),
        "issuer must be a did:web DID"
    );
    assert!(
        vc.id.starts_with("urn:uuid:"),
        "credential id must be urn:uuid"
    );

    let payload_hash = vc.credential_subject.payload_hash.as_str();
    assert_eq!(payload_hash.len(), 64, "SHA-256 hex is 64 chars");

    let canonical = crate::jws::canonical::canonicalize(&payload).unwrap();
    let expected_hash = hex::encode(sha2::Sha256::digest(&canonical));
    assert_eq!(payload_hash, expected_hash, "payload_hash must match");
}
