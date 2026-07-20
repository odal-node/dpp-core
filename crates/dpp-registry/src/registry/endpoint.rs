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
            // Update once the registry API specification is obtained — whether it is
            // publicly available is itself unconfirmed.
            api_version: "1.0".into(),
            mtls_required: false,
            token_endpoint: Some("https://sandbox.eudpp-registry.europa.eu/oauth2/token".into()),
        }
    }

    /// Create a production endpoint.
    ///
    /// ⚠️ **PROVISIONAL, and now known to be partly wrong.** The registry became
    /// operational on 20 July 2026 under Commission Implementing Regulation (EU)
    /// 2026/1778, but these constants predate it: all URLs, `api_version` and auth
    /// flows were guessed from the ESPR implementing acts and the DG GROW work
    /// programme.
    ///
    /// The **auth flow in particular rests on a wrong assumption** —
    /// `token_endpoint` models a bearer-token exchange, whereas registration
    /// identity is eIDAS-based (a verified operator proving identity by qualified
    /// electronic seal). That is a structural mismatch, not a wrong URL.
    ///
    /// Do NOT point this at real products. Reconciliation against the published
    /// specification is a breaking change scheduled for the next minor
    /// (COMPLIANCE-PIN PENDING).
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
