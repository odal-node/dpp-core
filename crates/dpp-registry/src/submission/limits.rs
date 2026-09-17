//! The bounds one submission is held to, and what backs each of them.
//!
//! 👁️ All three are **observed**, not specified — User Guide for Economic
//! Operators v1.02 (2026-08-24), chapter 6, which describes the web interface.
//! There is no published API specification, so whether the API enforces the
//! same numbers is unverified.

/// The most passports one submission may carry.
///
/// 👁️ User Guide v1.02: *"The system accepts up to 100 registration requests
/// per file. Files exceeding this limit cannot be processed."*
pub const MAX_PASSPORTS_PER_SUBMISSION: usize = 100;

/// The largest submission file the registry accepts, in bytes.
///
/// 👁️ User Guide v1.02: *"The file exceeds the maximum allowed size of 1 GB."*
///
/// 🚨 **The guide says "1 GB"; this is 1 GiB.** 1 073 741 824 against
/// 1 000 000 000 is a 7 % difference, and which one the registry means is not
/// stated. The binary reading is the common one for an upload limit and it is
/// still a reading — so the *observation* is the sentence, and the number is
/// ours. A submission between the two sizes is the case where that choice
/// decides the outcome.
///
/// # Nothing here enforces it, and that is deliberate
///
/// The limit is on an uploaded **file**. A [`RegistrationSubmission`](super::RegistrationSubmission) does not
/// know its own encoding — JSON or otherwise, pretty-printed or not, compressed
/// in transit or not — so a byte count taken here would measure a guess at what
/// the caller will send. Enforcement belongs to whatever serialises, which is
/// why [`fits_file_limit`] takes the encoded size rather than the submission.
pub const MAX_SUBMISSION_BYTES: u64 = 1_073_741_824;

/// Whether an encoded submission fits the registry's file limit.
///
/// Exists so the comparison is written once. The inclusivity is the part worth
/// not re-deriving per caller: the guide refuses a file that *"exceeds the
/// maximum allowed size"*, so exactly [`MAX_SUBMISSION_BYTES`] is accepted and
/// one byte more is not.
///
/// Takes the size rather than the submission because only the caller knows what
/// it encoded — see [`MAX_SUBMISSION_BYTES`].
#[must_use]
pub fn fits_file_limit(encoded_bytes: u64) -> bool {
    encoded_bytes <= MAX_SUBMISSION_BYTES
}

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
