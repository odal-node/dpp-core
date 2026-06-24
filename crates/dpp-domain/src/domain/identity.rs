//! Identity and access-tier types: `AccessTier`, `SignedCredential`, and `PassportCredential`.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// ESPR access tier levels for DPP data gating.
///
/// The EU ESPR mandates three tiers of data access:
/// - **Public**: No credential required. Visible to any consumer.
/// - **Professional**: Requires a W3C Verifiable Credential proving the
///   holder's role (repairer, recycler, remanufacturer, etc.).
/// - **Confidential**: Requires an institutional DID (market surveillance
///   authority, customs, notified body).
///
/// This is the canonical definition used across all dpp-core crates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AccessTier {
    Public = 0,
    Professional = 1,
    Confidential = 2,
}

/// A W3C Verifiable Credential 2.0 envelope binding a DPP passport to its signed payload.
///
/// The cryptographic proof is in [`SignedCredential::jws`]; this struct provides
/// the structured VC context required for EUDI/EBSI interoperability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportCredential {
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,
    /// Unique credential ID (`urn:uuid:…`) — generated fresh per signing call.
    pub id: String,
    /// DID of the signing issuer (`did:web:…`).
    pub issuer: String,
    /// Credential issuance timestamp (W3C VC 2.0 `validFrom`).
    pub valid_from: DateTime<Utc>,
    pub credential_subject: PassportCredentialSubject,
}

impl PassportCredential {
    /// W3C VCDM v2 base context — MUST be the first `@context` entry.
    pub const VC_BASE_CONTEXT: &'static str = "https://www.w3.org/ns/credentials/v2";
    /// Project-specific JSON-LD context for DPP passport credentials.
    pub const PASSPORT_CONTEXT: &'static str =
        "https://schema.odal-node.io/credentials/dpp-passport/v1";

    /// Construct a passport credential with the VCDM v2 base context and the
    /// `VerifiableCredential` base type guaranteed present, so a caller cannot
    /// emit a VC missing `https://www.w3.org/ns/credentials/v2`. `id`
    /// (`urn:uuid:` v7) and `valid_from` are generated fresh.
    #[must_use]
    pub fn new(issuer: String, credential_subject: PassportCredentialSubject) -> Self {
        Self {
            context: vec![Self::VC_BASE_CONTEXT.into(), Self::PASSPORT_CONTEXT.into()],
            credential_type: vec![
                "VerifiableCredential".into(),
                "DppPassportCredential".into(),
            ],
            id: format!("urn:uuid:{}", uuid::Uuid::now_v7()),
            issuer,
            valid_from: Utc::now(),
            credential_subject,
        }
    }
}

/// Claims about the DPP passport being attested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportCredentialSubject {
    /// `urn:uuid:{passport_id}` — the DPP passport being attested.
    pub id: String,
    /// SHA-256 hex digest of the RFC 8785 canonical payload bytes.
    pub payload_hash: String,
}

/// A DPP Verifiable Credential with its JWS proof signature.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SignedCredential {
    /// Structured W3C VC 2.0 passport credential.
    pub credential: PassportCredential,
    /// Compact JWS signature string (header.payload.signature).
    pub jws: String,
    /// The DID of the issuer (manufacturer or Odal on their behalf).
    pub issuer_did: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_passport_credential_guarantees_vc_base_context_and_type() {
        let vc = PassportCredential::new(
            "did:web:issuer.example.com".into(),
            PassportCredentialSubject {
                id: "urn:uuid:00000000-0000-0000-0000-000000000000".into(),
                payload_hash: "deadbeef".into(),
            },
        );
        // VCDM v2 requires the base context to be the first @context entry.
        assert_eq!(
            vc.context.first().map(String::as_str),
            Some(PassportCredential::VC_BASE_CONTEXT)
        );
        assert!(
            vc.credential_type
                .contains(&"VerifiableCredential".to_string())
        );
        assert!(vc.id.starts_with("urn:uuid:"));
    }
}
