//! [`RegistrationSubmission`] — the passports that travel and fail together.

use serde::{Deserialize, Serialize};

use super::MAX_PASSPORTS_PER_SUBMISSION;
use crate::error::RegistryValidationError;
use crate::payload::RegistrationPayload;

/// One act of submitting: between one and [`MAX_PASSPORTS_PER_SUBMISSION`]
/// passports, which the registry accepts or refuses **as a whole**.
///
/// # Why this exists rather than a bare `Vec`
///
/// 👁️ User Guide v1.02: *"When submitting multiple DPPs, if a single DPP has an
/// error, all the DPPs in the same submission will be rejected."* A submission
/// is therefore a unit with its own success, and a hundred passports that fail
/// produce **no records at all** — nothing for a per-passport status to describe,
/// and only the correlation identifier to hold on to.
///
/// This crate said all of that in prose and had no type for it: the envelope
/// carried exactly one payload, so [`MAX_PASSPORTS_PER_SUBMISSION`] and
/// `MAX_SUBMISSION_BYTES` were unenforceable and the all-or-nothing rule had
/// nothing to apply across. A consumer testing against that shape meets batch
/// rejection for the first time in production, with retry logic written for the
/// wrong failure unit.
///
/// # The bound is an invariant, on the way in and on the way back
///
/// The passports are private and [`new`](Self::new) is the only way to build a
/// multi-passport submission, so a value of this type is within the cap by
/// construction. Deserialisation is routed through the same check rather than
/// filling the field directly — a document is as much an input as a constructor
/// argument, and a stored submission of two hundred passports is one the
/// registry would refuse.
///
/// 🚨 The cap and the all-or-nothing rule are **observed, not specified** — User
/// Guide v1.02 (2026-08-24) chapter 6, describing the web interface. There is no
/// published API specification, and whether the API enforces the same numbers is
/// unverified. See [`RegistryBasis`](crate::RegistryBasis) for how this crate
/// records that distinction elsewhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(
    try_from = "Vec<RegistrationPayload>",
    into = "Vec<RegistrationPayload>"
)]
pub struct RegistrationSubmission {
    passports: Vec<RegistrationPayload>,
}

impl RegistrationSubmission {
    /// The common case: one passport, which cannot breach either bound.
    ///
    /// Infallible on purpose. Routing a single passport through
    /// [`new`](Self::new) would hand every caller a `Result` that cannot be
    /// `Err`, and a `Result` nobody can trigger teaches callers to `unwrap`.
    #[must_use]
    pub fn single(passport: RegistrationPayload) -> Self {
        Self {
            passports: vec![passport],
        }
    }

    /// A submission of one or more passports.
    ///
    /// # Errors
    ///
    /// [`RegistryValidationError::EmptySubmission`] if there are none — a
    /// submission that registers nothing is not a smaller submission, it is a
    /// request with no subject. [`RegistryValidationError::SubmissionTooLarge`]
    /// beyond [`MAX_PASSPORTS_PER_SUBMISSION`].
    ///
    /// The passports themselves are **not** validated here. That is
    /// [`validate`](Self::validate)'s job, and separating them keeps "this is a
    /// well-formed submission" answerable without deciding whether its contents
    /// would be accepted.
    pub fn new(passports: Vec<RegistrationPayload>) -> Result<Self, RegistryValidationError> {
        if passports.is_empty() {
            return Err(RegistryValidationError::EmptySubmission);
        }
        if passports.len() > MAX_PASSPORTS_PER_SUBMISSION {
            return Err(RegistryValidationError::SubmissionTooLarge {
                count: passports.len(),
                max: MAX_PASSPORTS_PER_SUBMISSION,
            });
        }
        Ok(Self { passports })
    }

    /// The passports this submission carries, in submission order.
    #[must_use]
    pub fn passports(&self) -> &[RegistrationPayload] {
        &self.passports
    }

    /// How many passports travel together.
    #[must_use]
    pub fn len(&self) -> usize {
        self.passports.len()
    }

    /// Always `false` — a submission carries at least one passport by
    /// construction. Present because clippy asks for it beside `len`, and
    /// because a caller reading it should see *why* rather than write a branch
    /// that can never be taken.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Validate every passport, refusing the **whole** submission at the first
    /// failure.
    ///
    /// # Errors
    ///
    /// [`RegistryValidationError::SubmissionPassportInvalid`], naming the index
    /// of the passport that failed and carrying its own error.
    ///
    /// 🚨 **Stops at the first failure, and that is the faithful behaviour.**
    /// Collecting every error would be friendlier and would misdescribe the
    /// registry: the outcome is refusal of the submission, not a list of
    /// per-passport verdicts, and a caller handed several errors will build a
    /// UI implying the others were fine. They were not accepted either.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        for (index, passport) in self.passports.iter().enumerate() {
            passport.validate().map_err(|source| {
                RegistryValidationError::SubmissionPassportInvalid {
                    index,
                    source: Box::new(source),
                }
            })?;
        }
        Ok(())
    }
}

impl TryFrom<Vec<RegistrationPayload>> for RegistrationSubmission {
    type Error = RegistryValidationError;

    fn try_from(passports: Vec<RegistrationPayload>) -> Result<Self, Self::Error> {
        Self::new(passports)
    }
}

impl From<RegistrationSubmission> for Vec<RegistrationPayload> {
    fn from(submission: RegistrationSubmission) -> Self {
        submission.passports
    }
}
