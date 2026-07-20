//! `dpp-registry` — EU Digital Product Passport Central Registry interface types.
//!
//! This crate models the data exchange with the EU Central Registry mandated by
//! ESPR Article 13. It provides request/response envelopes, error types, and
//! identifier structures.
//!
//! ⚠️ **These shapes predate the published specification.** The registry became
//! operational on 20 July 2026 under Commission Implementing Regulation (EU)
//! 2026/1778; the types here were derived from ESPR articles and JTC 24 draft
//! discussions before that, and are known to diverge from it in three respects:
//! no commodity code, no registration-granularity or identifier-linking concept
//! (IR Art. 8(1), (4), (5)), and a bearer-token authentication assumption where
//! registration rests on eIDAS verified-operator identity (IR Arts. 4–5). See
//! the EU Registry Readiness section of `docs/regulatory/COMPLIANCE.md`.
//! Reconciliation is a breaking change scheduled for the next minor. Do not
//! treat these as an implementation target.
//!
//! The crate is safe to compile for `wasm32-unknown-unknown` — it contains no
//! I/O, no HTTP clients, no async runtime. The platform repo provides the
//! actual HTTP adapter that implements network calls.
//!
//! # Key concepts
//!
//! - **Four persistent identifiers** (Article 13): every DPP must register a
//!   unique product identifier, product item identifier, facility identifier,
//!   and economic operator identifier.
//! - **Registration envelope**: the data payload sent to the EU registry when
//!   publishing or updating a DPP.
//! - **Status polling**: the registry returns a status that may be pending,
//!   registered, or rejected (with reasons).
//! - **Transfer notification**: when a transfer of responsibility occurs, the
//!   registry must be notified so it can update the responsible operator record.

pub mod registry;

pub use registry::{
    EuRegistryEnvelope, EuRegistryError, EuRegistryErrorKind, EuRegistryResponse,
    FacilityIdentifier, OperatorIdentifier, ProductIdentifier, ProductItemIdentifier,
    RegistrationPayload, RegistryAuthority, RegistryEndpoint, RegistryValidationError,
    StatusResponse, TransferNotification,
};
