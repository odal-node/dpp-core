//! `LocalIdentityService`: signing a passport and verifying it against the
//! node's own DID document.

use std::sync::Arc;

use serde_json::json;

use sha2::Digest;

use crate::local_service::LocalIdentityService;
use crate::test_support::temp_store;

use dpp_domain::{PassportId, ports::identity::IdentityPort};

fn test_service() -> LocalIdentityService {
    LocalIdentityService::new(
        Arc::new(temp_store("identity", "test-issuer")),
        "test-issuer".into(),
        "https://id.example.com".into(),
    )
}

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

/// Content-binding: a valid JWS signed over payload A must
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

/// `SignedCredential.credential` must be a proper W3C VC 2.0 envelope.
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

    let canonical = dpp_crypto::jws::canonical::canonicalize(&payload).unwrap();
    let expected_hash = hex::encode(sha2::Sha256::digest(&canonical));
    assert_eq!(payload_hash, expected_hash, "payload_hash must match");
}
