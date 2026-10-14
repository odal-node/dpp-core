//! [`RegistryEndpoint`] configuration, [`RegistryAuthority`], and the resource
//! paths beneath a base URL.
//!
//! # Where these paths come from, and what that is worth
//!
//! There is no published API specification. What exists is the registry's own
//! **unauthenticated web client**, which ships its endpoints as constants, and
//! reading it is how the `/api/v1` prefix was first learned. Re-read
//! **2026-09-16**, it also named the registration endpoint. Re-read
//! **2026-10-14**, together with the runtime configuration the client loads
//! (`assets/env-json-config.json`), it names the hosts the API is served from —
//! which are not the host the web interface is served from — and the dashboard
//! service a submission's outcome is read back from.
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
/// apart when the next reading updates some of them and not others. The
/// observations first made on 2026-09-16 were each found again on this date.
pub(crate) const LAST_READ: &str = "2026-10-14";

/// Registration submission, relative to [`RegistryEndpoint::base_url`].
///
/// 👁️ **Observed**, as `submitDppRegistrationRequest` in the registry's web
/// client. It replaces `/registrations`, which was invented and is now known to
/// be wrong rather than merely unverified.
///
/// The client's online form posts a JSON body of `productGroup` and
/// `uniqueProductIdentifier` and nothing else — it collects batch and model
/// identifiers and does not send them — and its file upload posts multipart
/// `productGroup` and `registrationFile`. The file's own schema sits behind the
/// registry's login, so [`EuRegistryEnvelope`](crate::EuRegistryEnvelope) is
/// still ours.
pub const REGISTRATION_PATH: &str = "/dpp-registration-requests";

/// The registry's product groups, relative to [`RegistryEndpoint::base_url`].
///
/// 👁️ **Observed** as `listProductGroups`. Each group carries a `code`, which
/// is what a registration names — see
/// [`registry_product_group`](crate::registry_product_group).
pub const PRODUCT_GROUPS_PATH: &str = "/product-groups";

/// The operator's tasks, relative to [`RegistryEndpoint::dashboard_base_url`].
///
/// 👁️ **Observed** as `listDashboardTasks`, paged (`page`, `size`, `sort`) and
/// answering with `content` and `totalElements`. A registration is a task of
/// type `DPP_REGISTRATION`, carrying a `taskId`, the `correlationId` the
/// registry's guide describes, and a `status`.
///
/// The client finds a submission's task by paging this list: no route that
/// takes a correlation identifier was seen, and the client does not read the
/// body the registration POST answers with, so what it returns is unobserved.
pub const TASKS_PATH: &str = "/tasks";

/// One task's result, relative to [`RegistryEndpoint::dashboard_base_url`].
///
/// `{id}` is the **task identifier**, not the correlation identifier.
///
/// 👁️ **Observed** as `getDashboardTaskResult`. For a registration it carries
/// the submission's `status` (see [`SubmissionOutcome`](crate::SubmissionOutcome))
/// and a `resultPayload` whose `report.itemDetails` lists each passport (see
/// [`PassportResult`](crate::PassportResult)). This is how the registry's own
/// interface reads a submission back, which is why no invented route stands
/// beside it.
pub const TASK_RESULT_PATH_TEMPLATE: &str = "/tasks/{id}/result";

/// The status of one registration, relative to [`RegistryEndpoint::base_url`].
///
/// `{id}` is the **registration identifier** the registry generated — IR (EU)
/// 2026/1778 Art. 8(8) — and never a caller's own passport identifier, which
/// the registry has no record of.
///
/// ⚠️ **Invented.** No constant for a status or polling route exists in the web
/// client, so unlike [`REGISTRATION_PATH`] this has nothing behind it. Kept
/// because a registered record has a status (Art. 10(1)) and a caller must be
/// able to name *a* path to read it; it is a placeholder, not an observation.
pub const STATUS_PATH_TEMPLATE: &str = "/registrations/{id}/status";

/// Transfer notification, relative to [`RegistryEndpoint::base_url`].
///
/// `{id}` is the registration identifier, as for [`STATUS_PATH_TEMPLATE`]: a
/// transfer moves a registered passport (Art. 6a), so it names the record.
///
/// ⚠️ **Invented**, on the same terms as [`STATUS_PATH_TEMPLATE`]. IR (EU)
/// 2026/1778 Art. 6a establishes that transfers happen; it says nothing about
/// the route they travel.
pub const TRANSFER_PATH_TEMPLATE: &str = "/registrations/{id}/transfer";

/// Where an authentication token is obtained, relative to the authentication
/// service's **host** — not to [`RegistryEndpoint::base_url`], which carries
/// the `/api/v1` prefix the API sits under.
///
/// ⚠️ **Invented.** IR (EU) 2026/1778 Art. 2(3) makes a token part of the
/// access model. The web client signs a person in through EU Login on a
/// separate authentication host, and that host is observed; a token for
/// machine access to the API is not, and nothing published says where one
/// comes from or how it is exchanged. Declared as a constant so that the route
/// has a stated basis like every other one, and a stand-in serving it can say
/// so.
pub const TOKEN_PATH: &str = "/oauth2/token";

/// The HTTP header carrying the request's idempotency key.
///
/// 👁️ **Observed.** The web client sets this header, with a UUID
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
///
/// A slice rather than an array, so that declaring a route does not change the
/// constant's type for every caller that names it.
pub const ENDPOINT_BASIS: &[(&str, RegistryBasis)] = &[
    (REGISTRATION_PATH, RegistryBasis::Observed { on: LAST_READ }),
    (
        PRODUCT_GROUPS_PATH,
        RegistryBasis::Observed { on: LAST_READ },
    ),
    (TASKS_PATH, RegistryBasis::Observed { on: LAST_READ }),
    (
        TASK_RESULT_PATH_TEMPLATE,
        RegistryBasis::Observed { on: LAST_READ },
    ),
    (STATUS_PATH_TEMPLATE, RegistryBasis::Assumed),
    (TRANSFER_PATH_TEMPLATE, RegistryBasis::Assumed),
    (TOKEN_PATH, RegistryBasis::Assumed),
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

/// Who answers at an endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum RegistryAuthority {
    /// EU Central DPP Registry (production).
    EuCentral,
    /// EU Sandbox / test environment.
    EuSandbox,
    /// National registry (member state specific).
    National(String),
    /// A stand-in a consumer runs itself, to exercise its own registration path
    /// end to end without the Commission's registry.
    ///
    /// 🚨 **Not an authority.** Nothing registered with a stand-in is
    /// registered, and a consumer must refuse to run against one wherever a
    /// registration would carry legal weight. It exists as a named variant so
    /// that a stand-in is never described as something it is not — the
    /// alternative is labelling it `National(..)` or `EuSandbox`, which states a
    /// relationship with a public authority that does not exist.
    StandIn,
}

/// Configuration for connecting to a specific EU registry endpoint.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RegistryEndpoint {
    /// Which authority this endpoint belongs to.
    pub authority: RegistryAuthority,
    /// Base URL of the registry API: registration, product groups, and the
    /// placeholder status and transfer routes.
    pub base_url: String,
    /// Base URL of the dashboard service, where a submission's outcome is read
    /// back as a task — [`TASKS_PATH`] and [`TASK_RESULT_PATH_TEMPLATE`].
    ///
    /// 👁️ A separate service in the registry's own runtime configuration, on
    /// the registry API's host under `/dashboard`.
    pub dashboard_base_url: String,
    /// API version supported (e.g. `"1.0"`).
    pub api_version: String,
    /// Whether mTLS is required.
    pub mtls_required: bool,
    /// Where an authentication token for the API is obtained, if applicable.
    ///
    /// ⚠️ **The token is the session, not the identity.** IR (EU) 2026/1778
    /// **Art. 2(3)** defines an *authentication token* as one used to prove
    /// authenticated sessions or delegated access between applications and the
    /// registry, and **Art. 18(1)(b)** stores such tokens as user credentials,
    /// so a token in front of the API is consistent with the act. Identity is a
    /// separate thing: the verified status of **Art. 4**, proved by qualified
    /// electronic seal or attestation and lapsing after at most three years. A
    /// token proves neither. How a token is *obtained* is unpublished, so this
    /// URL, and the client-credentials exchange an adapter may perform against
    /// it, are ours.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_endpoint: Option<String>,
}

impl RegistryEndpoint {
    /// Create a sandbox endpoint for development/testing.
    pub fn sandbox() -> Self {
        Self {
            authority: RegistryAuthority::EuSandbox,
            // 👁️ Observed in the test environment's runtime configuration
            // (`assets/env-json-config.json`, module `registry`). The address
            // the Commission publishes, `registry.acc.…`, serves the web
            // interface; the API it calls is on `reg.acc.…`. The `/api/v1`
            // prefix is the client's, and the paths beneath it are module
            // constants, each marked observed or invented.
            base_url: "https://reg.acc.product-passport.ec.europa.eu/api/v1".into(),
            // 👁️ Same configuration, module `dashboard`.
            dashboard_base_url: "https://reg.acc.product-passport.ec.europa.eu/dashboard/api/v1"
                .into(),
            // ⚠️ COMPLIANCE-PIN PENDING (watchlist 🟠): api_version "1.0" is provisional.
            // Update once the registry API specification is obtained — whether it is
            // publicly available is itself unconfirmed.
            api_version: "1.0".into(),
            mtls_required: false,
            // The host is the authentication service in the same configuration
            // (module `authentication`); the path is ours — see `TOKEN_PATH`.
            token_endpoint: Some(format!(
                "https://auth.acc.product-passport.ec.europa.eu{TOKEN_PATH}"
            )),
        }
    }

    /// Create a production endpoint.
    ///
    /// ⚠️ **PARTLY CONFIRMED, partly still provisional.** The registry became
    /// operational on 20 July 2026 under Commission Implementing Regulation (EU)
    /// 2026/1778. The **hosts** are the ones the registry's own interface calls,
    /// read from its runtime configuration; `api_version` and the auth flow are
    /// still inherited guesses.
    ///
    /// The **auth flow is half known.** A token in front of the API fits the act
    /// (IR 2026/1778 Art. 2(3)), but it is not identity: a verified operator
    /// proves identity by qualified electronic seal or qualified electronic
    /// attestation of attributes (Arts. 4–5), and how a token is obtained is
    /// unpublished — see [`token_endpoint`](Self::token_endpoint). Registration
    /// itself is performed over the registry's API (Art. 3(b)).
    ///
    /// Do NOT point this at real products. Reconciliation against the published
    /// specification is a breaking change scheduled for the next minor
    /// (COMPLIANCE-PIN PENDING).
    pub fn production() -> Self {
        Self {
            authority: RegistryAuthority::EuCentral,
            // 👁️ As for the sandbox, from the production configuration: the
            // published `registry.…` serves the interface, and the API it calls
            // is on `reg.…`.
            base_url: "https://reg.product-passport.ec.europa.eu/api/v1".into(),
            dashboard_base_url: "https://reg.product-passport.ec.europa.eu/dashboard/api/v1".into(),
            api_version: "1.0".into(),
            mtls_required: true,
            // Observed host, invented path, on the same terms as the sandbox's.
            token_endpoint: Some(format!(
                "https://auth.product-passport.ec.europa.eu{TOKEN_PATH}"
            )),
        }
    }

    /// An endpoint for a [`StandIn`](RegistryAuthority::StandIn) served at
    /// `origin` (scheme, host and port; a trailing slash is dropped).
    ///
    /// Shaped like the Commission's two endpoints — the API under `/api/v1`, the
    /// dashboard under `/dashboard/api/v1`, a token URL beside them, all on one
    /// origin here — so that a caller exercises the same path against a
    /// stand-in as it would against the registry. That sameness is the point
    /// and also the limit: a stand-in built from this crate's paths agrees with
    /// this crate, and says nothing about the registry. Check the
    /// [`RegistryBasis`] of each route it serves before reading a green run as
    /// more than that.
    #[must_use]
    pub fn stand_in(origin: &str) -> Self {
        let origin = origin.trim_end_matches('/');
        Self {
            authority: RegistryAuthority::StandIn,
            base_url: format!("{origin}/api/v1"),
            dashboard_base_url: format!("{origin}/dashboard/api/v1"),
            api_version: "1.0".into(),
            mtls_required: false,
            token_endpoint: Some(format!("{origin}{TOKEN_PATH}")),
        }
    }
}
