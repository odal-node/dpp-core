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
            // ✅ Host confirmed: the Commission publishes the registry's test
            // environment at this address (its "acc" sibling of the production
            // host). The earlier `sandbox.eudpp-registry.europa.eu` was invented
            // and resolves to nothing.
            //
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): the `/api/v1` prefix is
            // observed on the registry's own web client, not read from a
            // published specification, and the resource paths beneath it
            // (`/registrations`, …) remain guesses.
            base_url: "https://registry.acc.product-passport.ec.europa.eu/api/v1".into(),
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): api_version "1.0" is provisional.
            // Update once the registry API specification is obtained — whether it is
            // publicly available is itself unconfirmed.
            api_version: "1.0".into(),
            mtls_required: false,
            // ⚠️ Structurally wrong, kept only so the adapter compiles: registry
            // identity is eIDAS-based (qualified seal or qualified electronic
            // attestation of attributes), not an OAuth2 token exchange.
            token_endpoint: Some(
                "https://registry.acc.product-passport.ec.europa.eu/oauth2/token".into(),
            ),
        }
    }

    /// Create a production endpoint.
    ///
    /// ⚠️ **PARTLY CONFIRMED, partly still provisional.** The registry became
    /// operational on 20 July 2026 under Commission Implementing Regulation (EU)
    /// 2026/1778. The **host** is now the Commission's published one; the API
    /// path, `api_version` and auth flow are still inherited guesses.
    ///
    /// The **auth flow in particular rests on a wrong assumption** —
    /// `token_endpoint` models a bearer-token exchange, whereas registration
    /// identity is eIDAS-based: a verified operator proves identity by qualified
    /// electronic seal or qualified electronic attestation of attributes
    /// (IR 2026/1778 Arts. 4–5). That is a structural mismatch, not a wrong URL.
    /// Registration itself is performed over the registry's API (Art. 3(b)).
    ///
    /// Do NOT point this at real products. Reconciliation against the published
    /// specification is a breaking change scheduled for the next minor
    /// (COMPLIANCE-PIN PENDING).
    pub fn production() -> Self {
        Self {
            authority: RegistryAuthority::EuCentral,
            // ✅ Host confirmed: the Commission publishes the operational
            // registry at this address. The earlier `eudpp-registry.europa.eu`
            // was invented and resolves to nothing.
            //
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): as for the sandbox, the
            // `/api/v1` prefix is observed rather than specified, and the
            // resource paths beneath it remain guesses.
            base_url: "https://registry.product-passport.ec.europa.eu/api/v1".into(),
            api_version: "1.0".into(),
            mtls_required: true,
            // ⚠️ See the sandbox note: an OAuth2 token exchange is the wrong
            // model for eIDAS-based registry identity.
            token_endpoint: Some(
                "https://registry.product-passport.ec.europa.eu/oauth2/token".into(),
            ),
        }
    }
}
