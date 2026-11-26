//! An identity service that signs with a locally held key.

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine;
use sha2::{Digest, Sha256};

use dpp_domain::ports::identity::IdentityPort;
use dpp_domain::{DppError, PassportId, SignedCredential};

use dpp_crypto::jws::signer;
use dpp_crypto::jws::verifier::{resolve_verification_key, verify_jws};
use dpp_crypto::keystore::KeyBackend;

use crate::did_builder::{build_did_document, did_for};
use crate::passport_credential::build_passport_credential;

/// Concrete `IdentityPort` backed by a local [`KeyBackend`], such as a `KeyStore`.
///
/// Signs passports **in-process** with the store's signing key, under whichever
/// algorithm that key is recorded as using (Ed25519, ES256, ES384 or ES512), and
/// verifies JWS signatures against the issuer's published DID document. Holds the store
/// behind an `Arc` so one store can be shared with the did:web document endpoint: the
/// published key and the signing key are then the same store's, and there is no
/// network-reachable signing endpoint to attack.
///
/// `sign_passport` loads the key once and takes both the `kid` and the signature
/// from it, so a passport signed while its key rotates names the key that signed
/// it. A rotation that archives that key, rather than revoking it, leaves the
/// passport verifiable against the DID document published afterwards.
pub struct LocalIdentityService {
    store: Arc<dyn KeyBackend>,
    key_id: String,
    base_url: String,
}

impl LocalIdentityService {
    pub fn new(store: Arc<dyn KeyBackend>, key_id: String, base_url: String) -> Self {
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
        let canonical = dpp_crypto::jws::canonical::canonicalize(payload)
            .map_err(|e| DppError::Signing(e.to_string()))?;
        let payload_hash = hex::encode(Sha256::digest(&canonical));

        // The `kid` is the absolute DID URL of the signing key's verification
        // method, which is the identifier `build_did_document` gives that key. It
        // is taken from the key that signs, loaded once, so a rotation cannot come
        // between the two.
        let issuer_did = did_for(&self.base_url);
        let key = self
            .store
            .load_key(&self.key_id)
            .map_err(|e| DppError::Signing(e.to_string()))?;
        let kid = format!("{issuer_did}#{}", key.public_key().thumbprint_uri());

        let jws = signer::sign_typed_with(&key, payload, &kid, None, None)
            .map_err(|e| DppError::Signing(e.to_string()))?;

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

        let key = resolve_verification_key(&did_doc, jws)
            .ok_or_else(|| DppError::Signing("no matching public key in DID document".into()))?;

        if !verify_jws(jws, &key).map_err(|e| DppError::Signing(e.to_string()))? {
            return Ok(false);
        }

        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let signed_payload = match jws.split('.').nth(1).and_then(|p| b64.decode(p).ok()) {
            Some(bytes) => bytes,
            None => return Ok(false),
        };
        let expected = dpp_crypto::jws::canonical::canonicalize(payload)
            .map_err(|e| DppError::Signing(e.to_string()))?;
        Ok(signed_payload == expected)
    }

    async fn own_did_document(&self) -> Result<serde_json::Value, DppError> {
        build_did_document(&self.store, &self.base_url, &self.key_id)
            .map_err(|e| DppError::Signing(e.to_string()))
    }
}
