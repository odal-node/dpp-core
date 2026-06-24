//! Error type for all calculator operations in `dpp-calc`.

#[derive(Debug, thiserror::Error)]
pub enum CalcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// The ruleset's effective period has ended; a newer version is required.
    #[error("ruleset '{id}' expired on {until}")]
    RulesetExpired { id: String, until: String },

    /// The requested activity UUID is not present in the injected factor dataset.
    #[error("emission factor not found for activity '{0}'")]
    FactorNotFound(String),

    /// The supplied data cannot be processed by this methodology.
    #[error("methodology mismatch: {0}")]
    MethodologyMismatch(String),

    /// The methodology is defined but not yet implemented (gate: data license / delegated act).
    #[error("not implemented: {methodology} — {reason}")]
    NotImplemented { methodology: String, reason: String },

    /// A parameter combination that is internally incoherent per the ruleset.
    #[error("cross-field validation failed: {0}")]
    CrossFieldViolation(String),

    /// JSON canonicalization failed — inputs or outputs could not be serialized.
    #[error("canonicalization error: {0}")]
    CanonicalizeError(String),
}
