//! [`SealVerification`] — a verification outcome and how much was actually checked.

use super::indication::SealIndication;

/// How far validation got — what a [`SealIndication`] is actually founded on.
///
/// Without this, `TotalPassed` from two very different checks is one value. They
/// are not one claim: proving a signature is consistent with the certificate it
/// carries says nothing about whether that certificate was qualified, current,
/// or issued by anyone trustworthy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SealChecks {
    /// Nothing was checked — the verdict describes the envelope, not a
    /// validation. What a placeholder yields.
    None,
    /// The signature was checked against the certificate carried inside the
    /// seal, and nothing else: no certificate path, no revocation, no timestamp,
    /// no Trusted List.
    SignatureOnly,
    /// A full AdES validation: certificate path to a trust anchor, revocation
    /// status and timestamp, as well as the signature.
    FullValidation,
}

/// Result of verifying a `SealedEnvelope`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SealVerification {
    /// What the validation concluded.
    pub indication: SealIndication,
    /// What was actually checked to reach it.
    pub checks: SealChecks,
    /// True if this was a ghost/placeholder seal (always unverified in production).
    pub placeholder: bool,
}

impl SealVerification {
    /// A pass, founded on `checks`.
    ///
    /// Takes the checks rather than defaulting them, because "what was
    /// verified" is the whole content of a pass. There is no sensible default:
    /// guessing high overstates the claim and guessing low understates it.
    #[must_use]
    pub fn passed(checks: SealChecks) -> Self {
        Self {
            indication: SealIndication::TotalPassed,
            checks,
            placeholder: false,
        }
    }

    /// A definite negative — the seal does not attest what it claims to.
    #[must_use]
    pub fn failed(checks: SealChecks, reason: impl Into<String>) -> Self {
        Self {
            indication: SealIndication::TotalFailed(reason.into()),
            checks,
            placeholder: false,
        }
    }

    /// Validation did not fail, but there was not enough information to decide.
    ///
    /// The ordinary answer whenever material has to be fetched. Reach for this
    /// rather than [`Self::failed`] when the check did not complete: a sound
    /// passport reported non-compliant because a revocation endpoint was
    /// unreachable is a defect, not caution.
    #[must_use]
    pub fn indeterminate(checks: SealChecks, reason: impl Into<String>) -> Self {
        Self {
            indication: SealIndication::Indeterminate(reason.into()),
            checks,
            placeholder: false,
        }
    }

    /// The verdict for a placeholder envelope: nothing checked, nothing decided.
    ///
    /// Indeterminate rather than failed, and the distinction matters. A
    /// placeholder is not a seal that failed validation; it is a seal no
    /// validation was attempted on. Reporting it as failed would put a definite
    /// negative on a passport whose seal nobody has looked at.
    #[must_use]
    pub fn placeholder(reason: impl Into<String>) -> Self {
        Self {
            indication: SealIndication::Indeterminate(reason.into()),
            checks: SealChecks::None,
            placeholder: true,
        }
    }

    /// Whether this is a pass that a relying party may rest a compliance claim on.
    ///
    /// A named method because the mistake it prevents is the easy one to make:
    /// reading `TotalPassed` alone as "this is a valid qualified seal", when the
    /// check behind it may have been a bare signature comparison against a
    /// self-signed certificate. Requiring both parts at every call site would
    /// work exactly as well right up until one site forgot.
    #[must_use]
    pub fn is_qualified_pass(&self) -> bool {
        !self.placeholder
            && self.checks == SealChecks::FullValidation
            && self.indication == SealIndication::TotalPassed
    }

    /// Whether the verdict is internally consistent.
    ///
    /// One combination is incoherent: `TotalPassed` founded on
    /// [`SealChecks::None`] — a pass over nothing checked. That is not a
    /// stricter or looser claim than the others, it is a claim with no referent,
    /// and it is precisely the shape of the worst defect this port could ship: a
    /// verifier reporting a seal it never examined as good.
    ///
    /// `TotalFailed` with `None` is coherent, and deliberately so — an envelope
    /// can be rejected on its format before any validation is attempted.
    ///
    /// The fields are public, so this cannot be an unrepresentable state without
    /// a breaking redesign of a persisted value object. It is instead checkable,
    /// and [`crate::ports::seal::conformance`] checks it for every verdict an
    /// adapter produces.
    #[must_use]
    pub fn is_coherent(&self) -> bool {
        !(self.indication == SealIndication::TotalPassed && self.checks == SealChecks::None)
    }
}
