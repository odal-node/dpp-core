//! EU Central Registry types for ESPR Article 13 compliance.
//!
//! These types model the data exchanged with the EU EUDPP Central Registry.
//! The actual HTTP transport is implemented in the platform repo; this crate
//! only defines the shapes.
//!
//! ## Module layout
//!
//! - [`identifiers`] — the four Article 13 persistent identifiers (product,
//!   product item, facility, economic operator) — one vocabulary, one file.
//! - [`payload`] — [`RegistrationPayload`] and its [`EuRegistryEnvelope`].
//! - [`response`] — [`response::EuRegistryResponse`], [`response::StatusResponse`],
//!   [`response::RegistryStatusCode`].
//! - [`transfer`] — [`transfer::TransferNotification`].
//! - [`error`] — [`error::RegistryValidationError`], [`error::EuRegistryError`],
//!   [`error::EuRegistryErrorKind`].
//! - [`endpoint`] — [`endpoint::RegistryEndpoint`], [`endpoint::RegistryAuthority`]
//!   (keeps the ⚠️ COMPLIANCE-PIN block visible in one small file).

pub mod endpoint;
pub mod error;
pub mod identifiers;
pub mod payload;
pub mod response;
#[cfg(test)]
mod tests;
pub mod transfer;

pub use endpoint::{RegistryAuthority, RegistryEndpoint};
pub use error::{EuRegistryError, EuRegistryErrorKind, RegistryValidationError};
pub use identifiers::{
    FacilityIdentifier, OperatorIdentifier, ProductIdentifier, ProductItemIdentifier,
};
pub use payload::{EuRegistryEnvelope, RegistrationPayload};
pub use response::{EuRegistryResponse, RegistryStatusCode, StatusResponse};
pub use transfer::TransferNotification;
