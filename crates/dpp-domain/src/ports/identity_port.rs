//! `IdentityPort` — async trait for passport signing and JWS verification.

use async_trait::async_trait;

use crate::domain::{error::DppError, identity::SignedCredential, passport::PassportId};

/// Port trait for identity operations — signing and DID management.
#[async_trait]
pub trait IdentityPort: Send + Sync {
    /// Sign the canonical JSON payload for a passport, returning a compact JWS.
    async fn sign_passport(
        &self,
        passport_id: PassportId,
        payload: &serde_json::Value,
    ) -> Result<SignedCredential, DppError>;

    /// Verify a JWS signature against the issuer's published DID document.
    async fn verify_signature(
        &self,
        jws: &str,
        payload: &serde_json::Value,
    ) -> Result<bool, DppError>;

    /// Fetch this node's own current `did:web` document — the same document
    /// external verifiers resolve, snapshotted for embedding in an evidence
    /// dossier (`GET /dpp/{id}/evidence`) so offline verification never has
    /// to trust the issuing node again after export.
    async fn own_did_document(&self) -> Result<serde_json::Value, DppError>;
}
