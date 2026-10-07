//! SD-JWT — selective disclosure for JSON Web Tokens, IETF RFC 9901
//! (Standards Track, November 2025).
//!
//! An issuer replaces a claim's value with a salted hash and hands the cleartext
//! over separately as a *disclosure*. A holder forwards the signed token plus
//! only the disclosures it chooses. A verifier recomputes each disclosure's
//! digest and looks it up in the token; claims it was not given remain digests —
//! present, provably untampered, unreadable.
//!
//! # Why this is a primitive and not a credential
//!
//! Nothing here knows what a passport is, which fields are sensitive, or who may
//! see them. This module hides *whatever it is told to hide*. Deciding what to
//! hide is a disclosure-policy question and lives in `dpp-domain`; wrapping the
//! result in a credential is `dpp-vc`. The same split already separates
//! [`crate::jws`] from the things that sign with it.
//!
//! # What this module does not do
//!
//! - **It does not verify the issuer's signature.** [`SdJwt::parse`] checks the
//!   disclosure mechanism only. The JWS check is [`crate::jws::verifier`], and a
//!   caller must do both — a credential whose digests all match but whose
//!   signature is forged is worthless, and this module cannot tell.
//! - **`parse` does not authenticate the disclosures either**, which is the
//!   sharper half of the same point. It reads the serialisation; it does not
//!   compare what arrived against the signed `_sd` digests. A validly signed,
//!   entirely untouched JWT can be re-serialised with an extra forged
//!   disclosure, and [`SdJwt::disclosures`] will hand it back — the signature
//!   still verifies, because it never covered that list. Only
//!   [`SdJwt::disclosed_payload`] recomputes the digests, and it **refuses the
//!   whole token** rather than filtering: a disclosure matching no `_sd` digest
//!   is [`SdJwtError::UnusedDisclosures`], per clause 7.1 step 4. So **read
//!   claims from its output, never from `disclosures()`** — and read a failure
//!   from it as a tampered credential, not as a claim that was dropped.
//! - **It does not implement key binding.** RFC 9901 clause 4.3's KB-JWT proves
//!   the presenter holds a key the credential names. That needs holders to have
//!   keys. The parser tolerates a trailing KB-JWT segment so that a presentation
//!   carrying one is not misread as malformed, and otherwise ignores it.
//! - **It does not issue decoy digests** (clause 4.2.5). They are optional, and
//!   they buy concealment of *how many* claims or elements were withheld, which
//!   the digest count already reveals only in aggregate. Reading them works: a
//!   digest with no Disclosure is ignored, which is all a decoy is.
//!
//! # Array elements
//!
//! Clause 4.2.2 lets an issuer hide one element of an array and leave the rest.
//! The element's Disclosure is the two-element `[salt, value]`, and the array
//! carries `{"...": "<digest>"}` where the element was (clause 4.2.4.2).
//! [`Disclosure::element`] and [`conceal_elements`] make them. A verifier replaces
//! each placeholder it holds a Disclosure for with the value, **removes every
//! placeholder it does not**, and opens what it revealed for placeholders and
//! `_sd` arrays of its own (clause 7.1 step 3).
//!
//! Position is the one thing that is not hidden: the placeholders keep the order
//! of the array, because the order is the data. That is why [`conceal_elements`]
//! does not sort the way [`conceal`] does.
//!
//! # Two requirements that are easy to miss and are tested
//!
//! - **Digest order must not follow claim order.** Clause 4.2.4.1: *"The Issuer
//!   MUST hide the original order of the claims in the array."* Pushing digests
//!   in the order the fields were walked leaks the source structure, and a naive
//!   implementation does exactly that. [`conceal`] sorts.
//! - **A placeholder with no Disclosure is gone, not passed through.** Left in the
//!   array it would reach the caller as an object where a string was expected,
//!   which is a wrong answer and not a withheld one. Clause 7.1 step 3.d removes it.
//! - **A Disclosure of the wrong kind is an error.** A two-element Disclosure
//!   matched by an `_sd` digest, or a three-element one matched by a placeholder,
//!   is a token the issuer did not make (steps 3.c.ii.1 and 3.c.iii.1).
//! - **An unmatched disclosure is an error, not an omission.** Clause 7.1
//!   requires every disclosure to be used; one whose digest is absent from the
//!   token means the credential and the disclosures disagree, and the safe
//!   reading is refusal. [`SdJwt::disclosed_payload`] refuses.
//! - **A repeated digest is an error.** Clause 4.1: *"The same digest value MUST
//!   NOT appear more than once in the SD-JWT."* A map keyed by digest quietly
//!   satisfies a count-based check while collapsing the repeat, so the check is
//!   made against every occurrence rather than against what survived a
//!   deduplicating collection.
//! - **A presentation selects disclosures by digest, not by claim name.** One
//!   name can belong to several disclosures, so selecting by name reveals values
//!   the holder did not choose. See [`SdJwt::present`].

pub mod disclosure;
pub mod error;

mod builder;

#[cfg(test)]
mod array_tests;
#[cfg(test)]
mod rfc9901_array_tests;
#[cfg(test)]
mod rfc9901_example_tests;
#[cfg(test)]
mod rfc9901_examples;
#[cfg(test)]
mod rfc9901_vector_tests;
#[cfg(test)]
mod salt_tests;
#[cfg(test)]
mod tests;

pub use builder::{SdJwt, build_payload, conceal, conceal_elements};
pub use disclosure::{Disclosure, SD_HASH_ALG, digest_of};
pub use error::{DisclosureError, SdJwtError};
