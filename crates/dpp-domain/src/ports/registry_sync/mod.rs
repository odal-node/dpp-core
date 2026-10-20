//! Port trait for EU Central DPP Registry synchronisation.
//!
//! ESPR Article 13 establishes a central EU registry that stores at minimum
//! the unique identifiers for every product placed on the market. Its operating
//! rules are Commission Implementing Regulation (EU) 2026/1778.
//!
//! This port defines the interface a platform adapter implements. No registry
//! API specification is published, so the port is shaped by the regulation's
//! flow — submit, be checked, be registered — and not by any wire format. A
//! no-op `GhostRegistrySync` implementation is provided for testing and
//! development.

mod acknowledgement;
mod granularity;
mod lookup;
#[cfg(test)]
mod outcome_tests;
mod port;
mod record;
mod registering_operator;
mod report;
mod request;
mod service_provider;
mod status;
#[cfg(test)]
mod tests;
mod transfer_report;

pub use acknowledgement::SubmissionAcknowledgement;
pub use granularity::RegistrationGranularity;
pub use lookup::SubmissionLookup;
pub use port::RegistrySyncPort;
pub use record::RegistryRecord;
pub use registering_operator::RegisteringOperator;
pub use report::SubmissionReport;
pub use request::RegistrationRequest;
pub use service_provider::ServiceProviderRef;
pub use status::RegistryStatus;
pub use transfer_report::TransferReport;

/// No-op implementation for use before a deployment is enrolled with the EU
/// Central Registry.
///
/// Accepts every submission under a placeholder handle and knows of no
/// submission or record afterwards. Performs no network calls.
pub use crate::ports::ghosts::GhostRegistrySync;
