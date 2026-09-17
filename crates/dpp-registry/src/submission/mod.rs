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

/// The most passports one submission may carry.
///
/// 👁️ User Guide v1.02: *"The system accepts up to 100 registration requests
/// per file. Files exceeding this limit cannot be processed."*
pub const MAX_PASSPORTS_PER_SUBMISSION: usize = 100;

/// The largest submission file the registry accepts, in bytes.
///
/// 👁️ User Guide v1.02: *"The file exceeds the maximum allowed size of 1 GB."*
pub const MAX_SUBMISSION_BYTES: u64 = 1_073_741_824;

/// The longest unique product identifier the registry accepts, in characters.
///
/// 👁️ User Guide v1.02: the UPI is *"a mandatory value conforming to a URL
/// format compliant with JTC 24 standards. Max length is 2000 chars."*
///
/// 🚨 **This number moved, and it is the reason to distrust it.** v1.01
/// (2026-07-28) stated **50**, which would have been shorter than any GS1
/// Digital Link this workspace can build — a 65-character carrier URL, 77 with
/// a batch segment — and would have forced the carrier shape to change for
/// every printed label. v1.02 states 2000. The constraint was lifted by the
/// Commission within a month, silently, in a document with no OJ number and no
/// consolidation. Treat 2000 as current rather than settled.
pub const MAX_PRODUCT_IDENTIFIER_CHARS: usize = 2000;

mod outcome;
mod receipt;

#[cfg(test)]
mod tests;

pub use outcome::SubmissionOutcome;
pub use receipt::SubmissionReceipt;
