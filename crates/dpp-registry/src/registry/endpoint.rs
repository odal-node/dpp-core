//! [`RegistryEndpoint`] configuration and [`RegistryAuthority`].

use serde::{Deserialize, Serialize};

/// Known EU registry authority types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RegistryAuthority {
    /// EU Central DPP Registry (production).
    EuCentral,
    /// EU Sandbox / test environment.
    EuSandbox,
    /// National registry (member state specific).
    National(String),
}

/// Configuration for connecting to a specific EU registry endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEndpoint {
    /// Which authority this endpoint belongs to.
    pub authority: RegistryAuthority,
    /// Base URL of the registry API.
    pub base_url: String,
    /// API version supported (e.g. `"1.0"`).
    pub api_version: String,
    /// Whether mTLS is required.
    pub mtls_required: bool,
    /// OAuth2 / OIDC token endpoint, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<String>,
}

impl RegistryEndpoint {
    /// Create a sandbox endpoint for development/testing.
    pub fn sandbox() -> Self {
        Self {
            authority: RegistryAuthority::EuSandbox,
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): sandbox URL is an educated guess
            // based on the EC's EUDPP work programme. Confirm against the published sandbox
            // spec before enabling live calls. Track: ESPR implementing acts / DG GROW.
            base_url: "https://sandbox.eudpp-registry.europa.eu/api/v1".into(),
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): api_version "1.0" is provisional.
            // Update when the EU publishes the registry API specification.
            api_version: "1.0".into(),
            mtls_required: false,
            token_endpoint: Some("https://sandbox.eudpp-registry.europa.eu/oauth2/token".into()),
        }
    }

    /// Create a production endpoint.
    ///
    /// ⚠️ **PROVISIONAL**: The EU Central DPP Registry API has not been published
    /// as of 2026-06. All URLs, `api_version`, and auth flows are educated guesses
    /// based on the ESPR implementing acts and DG GROW work programme. Do NOT point
    /// this at real products until the Commission publishes the final spec and these
    /// constants are confirmed (COMPLIANCE-PIN PENDING).
    pub fn production() -> Self {
        Self {
            authority: RegistryAuthority::EuCentral,
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): placeholder URL — confirm
            // the real production endpoint from the published EU registry API spec.
            base_url: "https://eudpp-registry.europa.eu/api/v1".into(),
            api_version: "1.0".into(),
            mtls_required: true,
            token_endpoint: Some("https://eudpp-registry.europa.eu/oauth2/token".into()),
        }
    }
}
