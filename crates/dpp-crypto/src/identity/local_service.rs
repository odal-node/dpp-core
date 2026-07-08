//! In-process `IdentityPort` implementation backed by a local `KeyStore`.

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use sha2::{Digest, Sha256};

use dpp_domain::ports::identity_port::IdentityPort;
use dpp_domain::{DppError, PassportId, SignedCredential};

use crate::jws::signer;
use crate::jws::verifier::{
    extract_key_by_fingerprint, extract_kid_from_jws, extract_primary_public_key, verify_jws,
};
use crate::keystore::KeyStore;

use super::did_builder::build_did_document;
use super::passport_credential::build_passport_credential;

/// Concrete `IdentityPort` backed by a local `KeyStore`.
///
/// Signs passports **in-process** using Ed25519 keys from the store and verifies
/// JWS signatures against the issuer's published DID document. Holds the store
/// behind an `Arc` so one store can be shared with the did:web document endpoint
/// — signing and the published key are then guaranteed identical, and there is
/// no network-reachable signing endpoint to attack.
pub struct LocalIdentityService {
    store: Arc<KeyStore>,
    key_id: String,
    base_url: String,
}

impl LocalIdentityService {
    pub fn new(store: Arc<KeyStore>, key_id: String, base_url: String) -> Self {
        Self {
            store,
            key_id,
            base_url,
        }
    }
}

#[async_trait]
impl IdentityPort for LocalIdentityService {
    async fn sign_passport(
        &self,
        passport_id: PassportId,
        payload: &serde_json::Value,
    ) -> Result<SignedCredential, DppError> {
        let canonical = crate::jws::canonical::canonicalize(payload)
            .map_err(|e| DppError::Signing(e.to_string()))?;
        let payload_hash = hex::encode(Sha256::digest(&canonical));

        let jws = signer::sign(&self.store, &self.key_id, payload)
            .map_err(|e| DppError::Signing(e.to_string()))?;

        let did_doc = build_did_document(&self.store, &self.base_url, &self.key_id)
            .map_err(|e| DppError::Signing(e.to_string()))?;

        let issuer_did = did_doc["id"].as_str().unwrap_or_default().to_string();

        let passport_vc = build_passport_credential(issuer_did.clone(), passport_id, payload_hash);

        Ok(SignedCredential {
            credential: passport_vc,
            jws,
            issuer_did,
        })
    }

    async fn verify_signature(
        &self,
        jws: &str,
        payload: &serde_json::Value,
    ) -> Result<bool, DppError> {
        let did_doc = build_did_document(&self.store, &self.base_url, &self.key_id)
            .map_err(|e| DppError::Signing(e.to_string()))?;

        let pub_key_b64 = extract_kid_from_jws(jws)
            .and_then(|kid| extract_key_by_fingerprint(&did_doc, &kid))
            .or_else(|| extract_primary_public_key(&did_doc))
            .ok_or_else(|| DppError::Signing("no matching public key in DID document".into()))?;

        if !verify_jws(jws, &pub_key_b64).map_err(|e| DppError::Signing(e.to_string()))? {
            return Ok(false);
        }

        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let signed_payload = match jws.split('.').nth(1).and_then(|p| b64.decode(p).ok()) {
            Some(bytes) => bytes,
            None => return Ok(false),
        };
        let expected = crate::jws::canonical::canonicalize(payload)
            .map_err(|e| DppError::Signing(e.to_string()))?;
        Ok(signed_payload == expected)
    }

    async fn own_did_document(&self) -> Result<serde_json::Value, DppError> {
        build_did_document(&self.store, &self.base_url, &self.key_id)
            .map_err(|e| DppError::Signing(e.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::keystore::KeyStore;
    use dpp_domain::PassportId;
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
        let payload = serde_json::json!({ "productName": "Widget", "sector": "battery" });
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
}
