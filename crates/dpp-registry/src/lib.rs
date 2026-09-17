//! `dpp-registry` — EU Digital Product Passport Central Registry interface types.
//!
//! This crate models the data exchange with the EU Central Registry mandated by
//! ESPR Article 13. It provides request/response envelopes, error types, and
//! identifier structures.
//!
//! ⚠️ **One structural divergence remains, and it is the authentication model.**
//! [`RegistryEndpoint::token_endpoint`] models an OAuth2 bearer-token exchange.
//! Registration identity is eIDAS-based: a verified operator proves identity by
//! qualified electronic seal or qualified electronic attestation of attributes
//! (IR (EU) 2026/1778 Arts. 4–5). Those are different mechanisms and one does
//! not stand in for the other. The field exists so an adapter compiles, and is
//! marked wrong at its own definition. **Do not treat it as an implementation
//! target.**
//!
//! `api_version` is likewise our own construction. Endpoint paths are now a
//! mix: some are **observed** in the registry's own published web client, some
//! remain invented, and [`endpoint`] says which is which for each one. An
//! observation is better evidence than an invention and is still not a
//! specification — the registry's User Guide changed a stated identifier limit
//! by a factor of forty between two versions inside a month, and a web client
//! can move the same way.
//!
//! What the OJ text fixes *has* been reconciled against it — registration
//! granularity and identifier linking, commodity codes, the operator-identifier
//! scheme, asynchronous validation. The full account, item by item and split
//! between what is settled and what is still blocked on an unpublished API
//! specification, is the EU Registry Readiness section of
//! `docs/regulatory/COMPLIANCE.md`. It is not repeated here.
//!
//! That split is the point. This notice used to summarise three open
//! divergences; two were closed and the summary stayed as it was, telling
//! readers not to build against types that had since been fixed. A count is the
//! part that goes stale silently, so the count is gone and the one warning a
//! consumer must not miss is stated directly.
//!
//! The crate is safe to compile for `wasm32-unknown-unknown` — it contains no
//! I/O, no HTTP clients, no async runtime. The platform repo provides the
//! actual HTTP adapter that implements network calls.
//!
//! # Key concepts
//!
//! - **Persistent identifiers** (Annex III; Art. 13 stores them): every DPP registers a
//!   unique product identifier, product item identifier, facility identifier,
//!   and economic operator identifier.
//! - **Registration envelope**: the data payload sent to the EU registry when
//!   publishing or updating a DPP.
//! - **Status polling**: the registry returns a status that may be pending,
//!   registered, or rejected (with reasons).
//! - **Transfer notification**: when a transfer of responsibility occurs, the
//!   registry must be notified so it can update the responsible operator record.

//! # Module layout
//!
//! - [`identifiers`] — the four Article 13 persistent identifiers (product,
//!   product item, facility, economic operator) — one vocabulary, one file.
//! - [`granularity`] — [`Granularity`] and [`RegistrationLevel`]: the model /
//!   batch / item level a registration declares and the higher-level
//!   identifiers it must link.
//! - [`payload`] — [`RegistrationPayload`] and its [`EuRegistryEnvelope`].
//! - [`submission`] — [`RegistrationSubmission`]: the passports that travel and
//!   fail together, and the caps and outcome of one act of submitting.
//! - [`response`] — [`EuRegistryResponse`], [`StatusResponse`],
//!   [`RegistryStatusCode`].
//! - [`transfer`] — [`TransferNotification`].
//! - [`error`] — [`RegistryValidationError`], [`EuRegistryError`],
//!   [`EuRegistryErrorKind`].
//! - [`basis`] — [`RegistryBasis`]: whether a wire detail was observed on a
//!   date or is ours. The observed/invented split the constants state in prose,
//!   in a form a mock and a conformance test can act on.
//! - [`proof`] — [`ProofOfRegistration`]: the Art. 9 artefact evidencing that
//!   the registration obligation was discharged, and the Art. 9(4) ninety-day
//!   window the registry serves it for.
//! - [`endpoint`] — [`RegistryEndpoint`], [`RegistryAuthority`] (keeps the
//!   ⚠️ COMPLIANCE-PIN block visible in one small file).

pub mod basis;
pub mod endpoint;
pub mod error;
pub mod granularity;
pub mod identifiers;
pub mod payload;
pub mod proof;
pub mod response;
pub mod submission;
#[cfg(test)]
mod tests;
pub mod transfer;

pub use basis::RegistryBasis;
pub use endpoint::{
    ENDPOINT_BASIS, IDEMPOTENCY_KEY_HEADER, REGISTRATION_PATH, RegistryAuthority, RegistryEndpoint,
    STATUS_PATH_TEMPLATE, TRANSFER_PATH_TEMPLATE,
};
pub use error::{
    EuRegistryError, EuRegistryErrorKind, RegistryErrorBody, RegistryValidationError,
    STATUS_IDEMPOTENCY_KEY_REUSED, STATUS_IDEMPOTENCY_KEY_REUSED_BASIS,
    SUB_CODE_IDEMPOTENCY_KEY_REUSED,
};
pub use granularity::{Granularity, RegistrationLevel};
pub use identifiers::{
    FacilityIdentifier, OperatorIdentifier, PRODUCT_SCHEME_BASIS, ProductIdentifier,
    ProductItemIdentifier, SCHEME_DID, SCHEME_GTIN, SCHEME_IDENTIFICATION_LINK,
    ServiceProviderReference,
};
pub use payload::{EuRegistryEnvelope, RegistrationPayload};
pub use proof::{AVAILABILITY_DAYS, ProofOfRegistration};
pub use response::{EuRegistryResponse, RegistryStatusCode, StatusResponse};
pub use submission::{
    MAX_PASSPORTS_PER_SUBMISSION, MAX_PRODUCT_IDENTIFIER_CHARS, MAX_SUBMISSION_BYTES,
    RegistrationSubmission, SubmissionOutcome, SubmissionReceipt, fits_file_limit,
};
pub use transfer::TransferNotification;

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
