//! Compliance determination value objects — the result of running a product group's
//! rules against a passport, and the errors that stop one being produced.
//!
//! These are **persisted domain values**, not ports: they are serialised onto
//! `Passport::compliance_result` and travel on the wire. The traits that
//! produce them are the extension seam and live in
//! [`crate::ports::compliance`].

mod error;
mod finding;
mod result;
mod status;
#[cfg(test)]
mod status_all_tests;
#[cfg(test)]
mod tests;

pub use error::{ComplianceError, ComplianceErrorKind};
pub use finding::ComplianceFinding;
pub use result::ComplianceResult;
pub use status::{ComplianceStatus, gate_determination};
