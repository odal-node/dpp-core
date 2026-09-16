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
//! - **It does not implement key binding.** RFC 9901 clause 4.3's KB-JWT proves
//!   the presenter holds a key the credential names. That needs holders to have
//!   keys. The parser tolerates a trailing KB-JWT segment so that a presentation
//!   carrying one is not misread as malformed, and otherwise ignores it.
//! - **It does not implement array-element disclosures** (clause 4.2.2's `...`
//!   form) or decoy digests (clause 4.2.5). Both are optional. Array elements
//!   are not how any disclosure class here is expressed — a class attaches to a
//!   named field, and the whole array travels with it — and decoys buy
//!   concealment of *how many* claims were withheld, which the digest count
//!   already reveals only in aggregate.
//!
//! # Two requirements that are easy to miss and are tested
//!
//! - **Digest order must not follow claim order.** Clause 4.2.4.1: *"The Issuer
//!   MUST hide the original order of the claims in the array."* Pushing digests
//!   in the order the fields were walked leaks the source structure, and a naive
//!   implementation does exactly that. [`conceal`] sorts.
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
mod tests;

pub use builder::{SdJwt, build_payload, conceal};
pub use disclosure::{Disclosure, SD_HASH_ALG, digest_of};
pub use error::{DisclosureError, SdJwtError};
