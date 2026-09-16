//! Top-level error type for the DPP domain.

use thiserror::Error;

use crate::field_error::ValidationErrors;

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

    /// A stored document carries an envelope key this build has removed, so it
    /// predates a rename. Reading it would succeed while silently dropping the
    /// field, which is why it is refused instead. See
    /// [`crate::passport::REMOVED_ENVELOPE_KEYS`].
    #[error(
        "stored document carries removed envelope key `{removed}`, which was renamed to \
         `{replacement}`: reading it would silently drop the field, so it is refused"
    )]
    RemovedEnvelopeKey {
        /// The key as the stored document spells it.
        removed: &'static str,
        /// The key that replaced it.
        replacement: &'static str,
    },

    /// A stored document's product group data predates the current schema by more
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

    /// The forward walk from a passport to the record that replaced it could not
    /// produce one answer.
    ///
    /// Distinct from `Ok(None)`, which is the ordinary "nothing supersedes this"
    /// and is not an error. This is the store holding a shape the succession
    /// edge does not permit — two records claiming one predecessor, a cycle, or
    /// a chain longer than the walk will follow. Each is a reason the question
    /// has **no** answer rather than a negative one, and returning the
    /// first-sorted row instead would answer "which record am I holding" wrongly
    /// and silently.
    #[error("cannot resolve what supersedes passport {id}: {reason}")]
    SuccessionUnresolvable {
        /// The passport the walk started from.
        id: String,
        /// Which of the three shapes was met.
        reason: String,
    },

    #[error("internal error: {0}")]
    Internal(String),
}

impl From<ValidationErrors> for DppError {
    fn from(errors: ValidationErrors) -> Self {
        DppError::Validation(errors)
    }
}
