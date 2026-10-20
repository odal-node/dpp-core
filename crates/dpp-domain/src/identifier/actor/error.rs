//! [`ActorIdentifierError`] — why an operator or facility identifier was refused.

/// Why a value is not an admissible EN 18219 clause 6 identifier.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ActorIdentifierError {
    /// Not an ICD: four ASCII digits.
    #[error("an ICD is four ASCII digits, got '{0}'")]
    InvalidIcd(String),
    /// A value that is empty or only whitespace identifies no one.
    #[error("the identifier value is blank")]
    BlankValue,
    /// The value does not have the shape the scheme named by `icd` gives it.
    #[error("'{value}' is not a valid identifier under ICD {icd}")]
    InvalidValue {
        /// The ICD of the scheme the value was offered under.
        icd: String,
        /// The value as given.
        value: String,
    },
    /// Not a GS1 GLN: thirteen digits with a correct check digit.
    #[error("'{value}' is not a valid GLN: {reason}")]
    InvalidGln {
        /// The value as given.
        value: String,
        /// What was wrong with it.
        reason: String,
    },
    /// A legacy scheme string that no clause 6 scheme is known for in this build.
    ///
    /// Not a statement that the identifier is wrong. The scheme may have no
    /// clause 6 route at all, or have one this build has not established.
    #[error("scheme '{scheme}' has no EN 18219 clause 6 route in this build")]
    Unmapped {
        /// The legacy scheme string, as given.
        scheme: String,
    },
    /// A typed identifier in a scheme this build has no name for, which was read
    /// and kept but cannot be checked, so it is not issued.
    #[error("scheme '{scheme}' is not one this build knows, so it cannot be checked or issued")]
    UnrecognisedScheme {
        /// The scheme the identifier names, with its AI where it has one.
        scheme: String,
    },
}
