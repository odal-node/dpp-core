//! [`ComplianceError`] — why a compliance strategy or registry refused.

use std::fmt;

/// Error returned by a compliance strategy or registry.
#[derive(Debug, Clone)]
pub struct ComplianceError {
    pub kind: ComplianceErrorKind,
    pub message: String,
}

/// Classification of compliance calculation errors.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ComplianceErrorKind {
    /// No strategy registered for the requested product group.
    UnknownProductGroup,
    /// Input product group data is structurally invalid for this strategy.
    InvalidInput,
    /// Internal error; should not propagate to the user.
    Internal,
}

impl fmt::Display for ComplianceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for ComplianceError {}
