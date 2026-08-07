use chrono::Utc;
use serde_json::{Value, json};
use uuid::Uuid;

use super::types::{CredentialStatus, DppAccessCredential, DppCredentialSubject};

/// W3C VCDM v2 base context — MUST be the first `@context` entry.
const VC_BASE_CONTEXT: &str = "https://www.w3.org/ns/credentials/v2";

/// Inline JSON-LD term map for the DPP-specific terms this credential adds on
/// top of the VCDM v2 base context: the credential type value and the custom
/// subject properties. Inlined rather than hosted at a URL, for the same
/// reason as `dpp_vc::jsonld::context::passport_context` and
/// `dpp_domain::PassportCredential`: a string `@context` entry is fetched by
/// the consumer at expansion time, and this crate does not host a context
/// document. `dpp:` is a prefix IRI, never dereferenced during expansion.
fn dpp_terms() -> Value {
    json!({
        "dpp": "https://schema.odal-node.io/dpp#",
        "DppAccessCredential": "dpp:DppAccessCredential",
        "name": "dpp:name",
        "role": "dpp:role",
        "country": "dpp:country",
        "sectors": "dpp:sectors",
        "productCategories": "dpp:productCategories",
    })
}

/// Builder for constructing DPP access credentials.
pub struct CredentialBuilder {
    issuer_did: String,
    subject: DppCredentialSubject,
    expiration: chrono::DateTime<Utc>,
    status: Option<CredentialStatus>,
}

impl CredentialBuilder {
    /// Start building a credential from an issuer DID and subject claims.
    #[must_use]
    pub fn new(issuer_did: String, subject: DppCredentialSubject) -> Self {
        Self {
            issuer_did,
            subject,
            expiration: Utc::now() + chrono::Duration::days(365),
            status: None,
        }
    }

    /// Set the expiration date.
    #[must_use]
    pub fn expires_at(mut self, expiration: chrono::DateTime<Utc>) -> Self {
        self.expiration = expiration;
        self
    }

    /// Set the expiration to N days from now.
    #[must_use]
    pub fn expires_in_days(mut self, days: i64) -> Self {
        self.expiration = Utc::now() + chrono::Duration::days(days);
        self
    }

    /// Add a credential status for revocation checking.
    #[must_use]
    pub fn with_status(mut self, status: CredentialStatus) -> Self {
        self.status = Some(status);
        self
    }

    /// Build the unsigned credential.
    #[must_use]
    pub fn build(self) -> DppAccessCredential {
        DppAccessCredential {
            context: vec![json!(VC_BASE_CONTEXT), dpp_terms()],
            credential_type: vec!["VerifiableCredential".into(), "DppAccessCredential".into()],
            id: format!("urn:uuid:{}", Uuid::now_v7()),
            issuer: self.issuer_did,
            valid_from: Utc::now(),
            valid_until: self.expiration,
            credential_subject: self.subject,
            credential_status: self.status,
        }
    }
}
