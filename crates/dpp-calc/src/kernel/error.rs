//! Error type for all calculator operations in `dpp-calc`.

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum CalcError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// The ruleset's effective period has ended; a newer version is required.
    #[error("ruleset '{id}' expired on {until}")]
    RulesetExpired { id: String, until: String },

    /// The ruleset's effective period has a known start date that has not
    /// arrived yet.
    #[error("ruleset '{id}' is not yet effective (in force from {from})")]
    RulesetNotYetEffective { id: String, from: String },

    /// The ruleset has no application date at all, because the instrument that
    /// would date it has not entered into force. Distinct from
    /// [`RulesetNotYetEffective`](Self::RulesetNotYetEffective): that one knows
    /// the date and is waiting for it, this one cannot know it yet.
    #[error("ruleset '{id}' has no application date yet — awaiting {empowerment}")]
    RulesetUndetermined { id: String, empowerment: String },

    /// A computation overflowed to a non-finite value despite finite, in-range
    /// inputs — a legally cited figure must never silently become Infinity.
    #[error("calculation overflow: {0}")]
    Overflow(String),

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

    /// A bundle offered parameters for a ruleset whose numbers come from a
    /// published instrument. Fill, never override — see
    /// [`ParameterBasis`](crate::ruleset::ParameterBasis).
    #[error(
        "ruleset '{ruleset_id}' takes its parameters from the instrument it cites — a bundle may \
         fill parameters, never override ones that come from law"
    )]
    SourcedParametersNotFillable {
        /// The ruleset whose parameters were offered.
        ruleset_id: String,
    },

    /// A bundle offered a parameter group the ruleset does not declare.
    ///
    /// Refused rather than ignored: a dropped key reports success while
    /// changing nothing, which is the one failure an operator cannot see.
    #[error("ruleset '{ruleset_id}' has no parameter group '{name}' — it declares: {known}")]
    UnknownParameterGroup {
        /// The ruleset being filled.
        ruleset_id: String,
        /// The group name the bundle offered.
        name: String,
        /// The group names the ruleset does declare, comma-separated.
        known: String,
    },

    /// A bundle offered a parameter group whose JSON type differs from the
    /// ruleset's own.
    #[error(
        "ruleset '{ruleset_id}' parameter group '{name}' is {expected} in the ruleset but \
         {got} in the bundle"
    )]
    ParameterGroupTypeMismatch {
        /// The ruleset being filled.
        ruleset_id: String,
        /// The group name.
        name: String,
        /// The JSON type the ruleset holds.
        expected: &'static str,
        /// The JSON type the bundle offered.
        got: &'static str,
    },

    /// Parameters were read from an acceptance that no signature backs.
    ///
    /// A compiled-in baseline is a legitimate way to *start*, but not a source
    /// to take numbers from — see
    /// [`offered_for`](crate::parameters::offered_for).
    #[error(
        "refusing to take parameters for ruleset '{ruleset_id}' from an unverified acceptance — \
         only a bundle whose signature and content hash both checked may fill parameters"
    )]
    UnverifiedBundle {
        /// The ruleset whose parameters were being sought.
        ruleset_id: String,
    },
}
