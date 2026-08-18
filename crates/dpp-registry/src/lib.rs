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
//! target.** Endpoint paths and `api_version` are likewise our own construction,
//! each flagged at the point of use.
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
//! - [`response`] — [`EuRegistryResponse`], [`StatusResponse`],
//!   [`RegistryStatusCode`].
//! - [`transfer`] — [`TransferNotification`].
//! - [`error`] — [`RegistryValidationError`], [`EuRegistryError`],
//!   [`EuRegistryErrorKind`].
//! - [`endpoint`] — [`RegistryEndpoint`], [`RegistryAuthority`] (keeps the
//!   ⚠️ COMPLIANCE-PIN block visible in one small file).

pub mod endpoint;
pub mod error;
pub mod granularity;
pub mod identifiers;
pub mod payload;
pub mod response;
#[cfg(test)]
mod tests;
pub mod transfer;

pub use endpoint::{RegistryAuthority, RegistryEndpoint};
pub use error::{EuRegistryError, EuRegistryErrorKind, RegistryValidationError};
pub use granularity::{Granularity, RegistrationLevel};
pub use identifiers::{
    FacilityIdentifier, OperatorIdentifier, ProductIdentifier, ProductItemIdentifier,
};
pub use payload::{EuRegistryEnvelope, RegistrationPayload};
pub use response::{EuRegistryResponse, RegistryStatusCode, StatusResponse};
pub use transfer::TransferNotification;

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
