//! Top-level error type for the DPP domain.

use thiserror::Error;

use crate::domain::field_error::ValidationErrors;

/// Top-level error type for the DPP domain.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum DppError {
    #[error("passport not found: {0}")]
    NotFound(String),

    #[error(
        "passport is not in a state that allows this operation: current={current}, required={required}"
    )]
    InvalidTransition { current: String, required: String },

    #[error("validation failed: {0}")]
    Validation(ValidationErrors),

    #[error("signing failed: {0}")]
    Signing(String),

    #[error("serialisation error: {0}")]
    Serialisation(String),

    /// A stored document's sector data predates the current schema by more
    /// than the registered lens chain can bridge — e.g. a required field the
    /// document was written before, with no source data anywhere to derive it
    /// from. Not a bug to fix by writing a lens: some gaps have no honest
    /// transform. See [`crate::schemas::lens`].
    #[error("stored data does not match the current schema: {0}")]
    SchemaIncompatible(#[from] crate::schemas::lens::UpcastError),

    /// Returned when an attempt is made to delete or overwrite a passport that
    /// has been published and is therefore subject to EU ESPR retention obligations.
    /// Published passports must remain accessible for the legally defined period
    /// under the applicable delegated act (typically 10–15 years).
    #[error("passport is retention-locked: published passports cannot be deleted")]
    RetentionLocked,

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<ValidationErrors> for DppError {
    fn from(errors: ValidationErrors) -> Self {
        DppError::Validation(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_display() {
        let e = DppError::NotFound("passport-123".to_owned());
        assert_eq!(e.to_string(), "passport not found: passport-123");
    }

    #[test]
    fn invalid_transition_display() {
        let e = DppError::InvalidTransition {
            current: "archived".to_owned(),
            required: "draft".to_owned(),
        };
        let msg = e.to_string();
        assert!(
            msg.contains("archived"),
            "message should contain current state"
        );
        assert!(
            msg.contains("draft"),
            "message should contain required state"
        );
    }

    #[test]
    fn validation_display() {
        let e = DppError::Validation("product_name is required".into());
        assert!(e.to_string().contains("product_name is required"));
    }
}
