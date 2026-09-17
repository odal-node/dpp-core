//! [`RegistryEndpoint`] configuration, [`RegistryAuthority`], and the resource
//! paths beneath a base URL.
//!
//! # Where these paths come from, and what that is worth
//!
//! There is no published API specification. What exists is the registry's own
//! **unauthenticated web client**, which ships its endpoints as constants, and
//! reading it is how the `/api/v1` prefix was first learned. Re-read
//! **2026-09-16**, it also names the registration endpoint.
//!
//! That is better evidence than an invention and it is **not a specification**.
//! It is one consumer's behaviour on one date, and a client can change without
//! notice — the registry's own User Guide changed a stated identifier limit by a
//! factor of forty between two versions in under a month. So each constant below
//! says which of the two it is, and nothing here should be read as *the registry
//! requires*.

use serde::{Deserialize, Serialize};

use crate::basis::RegistryBasis;

/// The date the registry's web client was last read.
///
/// One constant rather than a literal per entry: every observation below came
/// from the same reading, and repeating the date invites the entries to drift
/// apart when the next reading updates some of them and not others.
const LAST_READ: &str = "2026-09-16";

/// Registration submission, relative to [`RegistryEndpoint::base_url`].
///
/// 👁️ **Observed 2026-09-16**, as `submitDppRegistrationRequest` in the
/// registry's web client. It replaces `/registrations`, which was invented and
/// is now known to be wrong rather than merely unverified.
pub const REGISTRATION_PATH: &str = "/dpp-registration-requests";

/// Registration status polling, relative to [`RegistryEndpoint::base_url`].
///
/// ⚠️ **Invented.** No constant for a status or polling route exists in the web
/// client, so unlike [`REGISTRATION_PATH`] this has nothing behind it. Kept
/// because the asynchronous flow needs *a* path and a caller must be able to
/// name one; it is a placeholder, not an observation.
pub const STATUS_PATH_TEMPLATE: &str = "/registrations/{id}/status";

/// Transfer notification, relative to [`RegistryEndpoint::base_url`].
///
/// ⚠️ **Invented**, on the same terms as [`STATUS_PATH_TEMPLATE`]. IR (EU)
/// 2026/1778 Art. 6a establishes that transfers happen; it says nothing about
/// the route they travel.
pub const TRANSFER_PATH_TEMPLATE: &str = "/registrations/{id}/transfer";

/// The HTTP header carrying the request's idempotency key.
///
/// 👁️ **Observed 2026-09-16.** The web client sets this header, with a UUID
/// value, on every POST to registration and enrolment. A replayed key returns
/// **409** with `subCode` `CONFLICT_IDEMPOTENCY_KEY_ALREADY_USED`, and the
/// client deliberately *keeps* the key on that response rather than minting a
/// new one — it reads a conflict as "this already succeeded", not as "try
/// again differently".
///
/// This is a **header**, which is the correction it carries: idempotency was
/// previously assumed to ride on the envelope body. See
/// [`EuRegistryEnvelope::request_id`](crate::EuRegistryEnvelope::request_id),
/// which is ours and is a different thing.
pub const IDEMPOTENCY_KEY_HEADER: &str = "Idempotency-Key";

/// Every wire constant this module declares, with what backs it.
///
/// The same observed/invented split the constants state in prose, in a form
/// code can act on — a mock can report the basis of each route it serves, and a
/// test can assert it exercised nothing [`RegistryBasis::Assumed`]. See
/// [`crate::basis`] for why that matters.
///
/// 🚨 **This table is the contract, and a test holds it to the constants.** A
/// path added to this module and not to the table would be a route with no
/// stated provenance, which is the state the module was in before the table
/// existed — so `every_wire_constant_declares_a_basis` fails until both agree.
pub const ENDPOINT_BASIS: [(&str, RegistryBasis); 4] = [
    (REGISTRATION_PATH, RegistryBasis::Observed { on: LAST_READ }),
    (STATUS_PATH_TEMPLATE, RegistryBasis::Assumed),
    (TRANSFER_PATH_TEMPLATE, RegistryBasis::Assumed),
    (
        IDEMPOTENCY_KEY_HEADER,
        RegistryBasis::Observed { on: LAST_READ },
    ),
];

/// What backs `value`, or `None` if this module does not declare it.
///
/// `None` is not "invented" — it is "this crate never said this". A caller
/// getting `None` for a route it believes it took from here has a typo or a
/// stale copy, and answering `Assumed` would hide that behind a plausible one.
#[must_use]
pub fn basis_of(value: &str) -> Option<RegistryBasis> {
    ENDPOINT_BASIS
        .iter()
        .find(|(declared, _)| *declared == value)
        .map(|(_, basis)| *basis)
}

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
            // 👁️ The `/api/v1` prefix is observed on the registry's own web
            // client — re-confirmed 2026-09-16 — not read from a published
            // specification. The resource paths beneath it are module
            // constants, each marked observed or invented.
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
            // 👁️ As for the sandbox: the `/api/v1` prefix is observed rather
            // than specified, and the paths beneath it are module constants.
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
