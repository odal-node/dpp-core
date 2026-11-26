//! Tests of the `local_service` module.

use super::super::*;
use dpp_crypto::keystore::KeyStore;
use dpp_domain::PassportId;
use dpp_domain::ports::identity::IdentityPort;
use std::sync::Arc;

fn service() -> LocalIdentityService {
    let path = std::env::temp_dir().join(format!("ls-test-{}.json", uuid::Uuid::new_v4()));
    let store = KeyStore::open(&path, "test-pass").expect("open store");
    store.generate_key("root").expect("generate key");
    LocalIdentityService::new(
        Arc::new(store),
        "root".to_owned(),
        "https://node.example.com".to_owned(),
    )
}

#[tokio::test]
async fn signs_then_verifies_round_trip() {
    let svc = service();
    let payload = serde_json::json!({ "productName": "Widget", "productGroup": "battery" });
    let signed = svc
        .sign_passport(PassportId::new(), &payload)
        .await
        .expect("sign");

    assert!(!signed.jws.is_empty());
    assert!(signed.issuer_did.starts_with("did:web:"));
    assert_eq!(signed.credential.issuer, signed.issuer_did);

    let ok = svc
        .verify_signature(&signed.jws, &payload)
        .await
        .expect("verify");
    assert!(ok, "freshly signed payload must verify");
}

#[tokio::test]
async fn verify_rejects_tampered_payload() {
    let svc = service();
    let payload = serde_json::json!({ "value": 1 });
    let signed = svc
        .sign_passport(PassportId::new(), &payload)
        .await
        .expect("sign");

    let tampered = serde_json::json!({ "value": 2 });
    let ok = svc
        .verify_signature(&signed.jws, &tampered)
        .await
        .expect("verify");
    assert!(!ok, "a tampered payload must not verify");
}
