//! Port trait for EU Central DPP Registry synchronisation.
//!
//! ESPR Article 13 establishes a central EU registry that stores at minimum
//! the unique identifiers for every product placed on the market. The registry
//! is scheduled to go live on 19 July 2026.
//!
//! This port defines the interface that platform adapters implement once the
//! Commission publishes the registry API specification. Until then, a no-op
//! `GhostRegistrySync` implementation is provided for testing and development.

mod granularity;
mod identifiers;
mod port;
mod record;
mod registering_operator;
mod request;
mod status;
#[cfg(test)]
mod tests;

pub use granularity::RegistrationGranularity;
pub use identifiers::RegistryIdentifiers;
pub use port::RegistrySyncPort;
pub use record::RegistryRecord;
pub use registering_operator::RegisteringOperator;
pub use request::RegistrationRequest;
pub use status::RegistryStatus;

/// No-op implementation for use before the EU Central Registry API is published.
///
/// Returns synthetic records with `RegistryStatus::Pending` and placeholder
/// identifiers. All operations succeed but perform no real network calls.
pub use crate::ports::ghosts::GhostRegistrySync;
