//! Tests that DID document and verification paths do not write to the key store.

use super::super::super::*;
use super::support::CountingKeyBackend;
use dpp_crypto::keystore::KeyBackend;
use dpp_domain::PassportId;
use dpp_domain::ports::identity::IdentityPort;
use std::sync::{Arc, atomic::Ordering};

#[tokio::test]
async fn document_and_verification_paths_never_write_keys() {
    let backend = Arc::new(CountingKeyBackend::default());
    let base_url = "https://node.example.com";

    assert!(
        crate::did_builder::build_did_document(backend.as_ref(), base_url, "root").is_err(),
        "building a document without a key must fail"
    );
    let uninitialized =
        LocalIdentityService::new(backend.clone(), "root".to_owned(), base_url.to_owned());
    assert!(uninitialized.own_did_document().await.is_err());
    assert!(
        uninitialized
            .verify_signature("not-a-jws", &serde_json::json!({}))
            .await
            .is_err()
    );
    assert_eq!(backend.writes.load(Ordering::SeqCst), 0);
    assert!(!backend.has_key("root"));

    backend.generate_key("root").expect("provision issuer key");
    let writes_after_provisioning = backend.writes.load(Ordering::SeqCst);
    let service =
        LocalIdentityService::new(backend.clone(), "root".to_owned(), base_url.to_owned());
    service
        .own_did_document()
        .await
        .expect("build DID document");
    let payload = serde_json::json!({"product": "widget"});
    let signed = service
        .sign_passport(PassportId::new(), &payload)
        .await
        .expect("sign passport");
    assert!(
        service
            .verify_signature(&signed.jws, &payload)
            .await
            .expect("verify signature")
    );
    assert_eq!(
        backend.writes.load(Ordering::SeqCst),
        writes_after_provisioning,
        "document and verification paths must not write after startup provisioning"
    );
}
