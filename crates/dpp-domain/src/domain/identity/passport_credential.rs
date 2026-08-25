//! [`PassportCredential`] — the W3C VC 2.0 envelope around a signed passport.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::credential_subject::PassportCredentialSubject;

/// A W3C Verifiable Credential 2.0 envelope binding a DPP passport to its signed payload.
///
/// The cryptographic proof is in [`SignedCredential::jws`](crate::domain::identity::SignedCredential::jws); this struct provides
/// the structured VC context required for EUDI/EBSI interoperability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportCredential {
    #[serde(rename = "@context")]
    pub context: Vec<Value>,
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

    /// Inline JSON-LD term map for the DPP-specific terms this credential adds
    /// on top of the VCDM v2 base context: the credential type value and the
    /// one custom subject property (`payloadHash`).
    ///
    /// Inlined rather than hosted at a URL — a string entry in `@context` is
    /// fetched by the consumer at expansion time, and this crate does not host
    /// a context document. Same reasoning and `dpp:` prefix as
    /// `dpp_vc::jsonld::context::passport_context`; a prefix IRI names a
    /// vocabulary and is never dereferenced during expansion, so it carries no
    /// such obligation.
    fn dpp_terms() -> Value {
        json!({
            "dpp": "https://schema.odal-node.io/dpp#",
            "DppPassportCredential": "dpp:DppPassportCredential",
            "payloadHash": "dpp:payloadHash",
        })
    }

    /// Construct a passport credential with the VCDM v2 base context and the
    /// `VerifiableCredential` base type guaranteed present, so a caller cannot
    /// emit a VC missing `https://www.w3.org/ns/credentials/v2`. `id`
    /// (`urn:uuid:` v7) and `valid_from` are generated fresh.
    #[must_use]
    pub fn new(issuer: String, credential_subject: PassportCredentialSubject) -> Self {
        Self {
            context: vec![json!(Self::VC_BASE_CONTEXT), Self::dpp_terms()],
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
