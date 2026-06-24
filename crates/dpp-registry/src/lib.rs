//! `dpp-registry` — EU Digital Product Passport Central Registry interface types.
//!
//! This crate models the data exchange with the EU Central Registry mandated by
//! ESPR Article 13. It provides request/response envelopes, error types, and
//! identifier structures aligned with the anticipated EU API specification.
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
