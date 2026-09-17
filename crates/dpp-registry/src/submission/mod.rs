//! What becomes of a *submission*, as distinct from what becomes of a *record*.
//!
//! # Two different questions, previously one enum
//!
//! [`RegistryStatusCode`](crate::RegistryStatusCode) answers *"what is the
//! state of this registered passport?"* — registered, suspended, deactivated.
//! That is a property of a DPP that exists in the registry.
//!
//! [`SubmissionOutcome`] answers *"what happened to the request I sent?"* — it
//! is still validating, it succeeded, it failed. That is a property of an
//! **act of submitting**, which may carry many passports and may fail as a
//! whole.
//!
//! Collapsing the two loses the case that motivates the distinction: a
//! submission of a hundred passports that fails validation produces **no
//! records at all**, so there is nothing for a record status to describe, and
//! the only thing a caller can hold on to is the correlation identifier the
//! registry returned. A single enum forces that state to be expressed as a
//! per-passport status that the registry never assigned.
//!
//! # Provenance
//!
//! 👁️ **Observed**, not specified, from the registry's *User Guide for
//! Economic Operators* v1.02 (2026-08-24), chapter 6. There is no published API
//! specification. The guide describes the web interface, so these are the
//! states that interface reports and the limits it enforces; whether the API
//! names them identically is unverified.

mod batch;
#[cfg(test)]
mod batch_tests;
mod limits;
mod outcome;
mod receipt;

#[cfg(test)]
mod tests;

pub use batch::RegistrationSubmission;
pub use limits::{
    MAX_PASSPORTS_PER_SUBMISSION, MAX_PRODUCT_IDENTIFIER_CHARS, MAX_SUBMISSION_BYTES,
    fits_file_limit,
};
pub use outcome::SubmissionOutcome;
pub use receipt::SubmissionReceipt;
