use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Re-export the canonical `AccessTier` from dpp-domain.
pub use dpp_domain::AccessTier;

// ─── Credential role ─────────────────────────────────────────────────────────

/// The access role granted by a Verifiable Credential.
///
/// Maps to the ESPR access tiers and the specific operator roles
/// defined in the transfer-of-responsibility model.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialRole {
    /// Authorised repairer — can access disassembly instructions, spare parts info.
    AuthorisedRepairer,
    /// Recycler — can access material composition, SVHC data, recycling instructions.
    Recycler,
    /// Remanufacturer — can access full technical specifications.
    Remanufacturer,
    /// Preparer for reuse — can access quality and safety data.
    PreparerForReuse,
    /// Distributor with professional access.
    Distributor,
    /// Market surveillance authority — full access to all tiers.
    MarketSurveillanceAuthority,
    /// Customs authority — access for border control.
    CustomsAuthority,
    /// Notified body — conformity assessment.
    NotifiedBody,
    /// Custom role (extension point for sector-specific roles).
    Custom(String),
}

impl CredentialRole {
    /// Returns the minimum access tier this role grants.
    pub fn access_tier(&self) -> AccessTier {
        match self {
            Self::MarketSurveillanceAuthority | Self::CustomsAuthority | Self::NotifiedBody => {
                AccessTier::Confidential
            }
            _ => AccessTier::Professional,
        }
    }
}

// ─── Credential subject ─────────────────────────────────────────────────────

/// The claims inside a DPP access credential.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DppCredentialSubject {
    /// DID of the credential holder (the entity being granted access).
    pub id: String,
    /// Legal name of the credential holder.
    pub name: String,
    /// The role being granted.
    pub role: CredentialRole,
    /// ISO 3166-1 alpha-2 country code of the holder's registration.
    pub country: String,
    /// Sector(s) this credential applies to (e.g., `["textile"]`).
    /// Empty means all sectors.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub sectors: Vec<String>,
    /// Specific product categories this credential covers.
    /// Empty means all categories within the sectors.
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub product_categories: Vec<String>,
}

// ─── Verifiable Credential envelope ─────────────────────────────────────────

/// A W3C Verifiable Credential for DPP access.
///
/// Follows the VC Data Model v2.0 structure with DPP-specific extensions.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DppAccessCredential {
    /// JSON-LD context (always includes the VC context).
    #[serde(rename = "@context")]
    pub context: Vec<String>,
    /// Credential type (always includes "VerifiableCredential").
    #[serde(rename = "type")]
    pub credential_type: Vec<String>,
    /// Unique credential ID (UUID-based URI).
    pub id: String,
    /// DID of the issuer (the authority granting access).
    pub issuer: String,
    /// When the credential was issued.
    pub valid_from: DateTime<Utc>,
    /// When the credential expires (mandatory for DPP credentials).
    pub valid_until: DateTime<Utc>,
    /// The access claims.
    pub credential_subject: DppCredentialSubject,
    /// Credential status for revocation checking.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub credential_status: Option<CredentialStatus>,
}

/// Credential status descriptor for revocation checking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CredentialStatus {
    /// URL to check revocation status.
    pub id: String,
    /// Status method type. Per W3C Bitstring Status List v1.0 this is
    /// `"BitstringStatusListEntry"` (the older `"StatusList2021Entry"` is dated).
    #[serde(rename = "type")]
    pub status_type: String,
    /// Index in the status list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_list_index: Option<String>,
    /// URL of the status list credential.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_list_credential: Option<String>,
}
