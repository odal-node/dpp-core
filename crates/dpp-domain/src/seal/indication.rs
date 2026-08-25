//! [`SealIndication`] — the ETSI verification outcome a check reports.

/// The outcome of validating a seal.
///
/// Three-valued, because AdES validation is. The names follow the status
/// indications in **ETSI EN 319 102-1**, the standard that specifies how an AdES
/// signature is validated, so a verdict produced here maps onto one produced by
/// any conformant validator without a translation step that could lose its
/// meaning.
///
/// The middle value is the reason this is not a boolean.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SealIndication {
    /// The seal passed the validation that was performed.
    ///
    /// What that is worth depends entirely on [`SealChecks`](crate::seal::SealChecks) — a pass over a
    /// signature check alone is a far smaller claim than a pass over a full
    /// validation, and the two must never be read as the same statement.
    TotalPassed,

    /// The format is wrong, or the signature value failed verification.
    ///
    /// A definite negative: this seal does not attest what it claims to.
    TotalFailed(String),

    /// Validation did not fail, but there was not enough information to decide.
    ///
    /// The ordinary answer whenever material has to be fetched — revocation data
    /// unreachable, a timestamp not yet corroborated, a trust anchor
    /// unresolvable at the moment of asking. It means *ask again later*, not
    /// *reject this passport*, and collapsing it either way is wrong: to failed,
    /// and a sound passport is reported non-compliant; to passed, and a check
    /// that never completed is claimed as one that did.
    Indeterminate(String),
}
