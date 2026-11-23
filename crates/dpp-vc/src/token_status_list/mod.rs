//! Token Status List: how a credential is withdrawn after it has been issued.
//!
//! Implements the list, the reference a credential carries to it, and the check a
//! verifier makes, of **draft-ietf-oauth-status-list-21** (Token Status List), read
//! on 2026-11-23. It is what the SD-JWT VC profile names for its `status` claim
//! (draft-ietf-oauth-sd-jwt-vc-19 clause 2.2.2.3), and
//! [`sd_jwt_vc::issue`](crate::sd_jwt_vc::issue) writes the reference.
//!
//! **No conformance is claimed.** The document is an Internet-Draft, not an RFC, and
//! until it is one nothing here is a claim about the standard: only about which
//! revision was read, and when. It is also not the W3C Bitstring Status List of
//! [`crate::status_list`], which is a different format on a different carrier: the
//! two differ in compression (zlib, not GZIP), in the width of an entry (1, 2, 4 or
//! 8 bits, not 1) and in the order the bits are counted in.
//!
//! # The shape
//!
//! 1. The issuer puts a [`StatusReference`] in the credential: an `idx` and the
//!    `uri` of a list.
//! 2. The status issuer publishes a Status List Token at that `uri`: a JWT, typed
//!    [`TYP`], whose `sub` is the `uri` and whose `status_list` holds a
//!    [`TokenStatusList`], compressed.
//! 3. A verifier fetches it, checks it with [`StatusListToken::verify`], and reads
//!    the credential's entry with [`StatusListToken::status`]: [`Status::Valid`],
//!    [`Status::Revoked`] or [`Status::Suspended`].
//!
//! # Not here
//!
//! - **Fetching, caching and publishing.** Each needs the network or a host, and
//!   this crate has neither. [`StatusListToken::ttl`] hands the verifier the cache
//!   rule of section 8.3 step 4d.
//! - **Signing a Status List Token.** A status issuer signs the JWT with the
//!   [`TokenStatusList::encode`] output as `lst`.
//! - **The CBOR and COSE form**, and **MAC-secured tokens.** Only the JWT form,
//!   secured by a signature, is read.
//! - **Which key may speak for a credential.** The draft leaves key resolution out
//!   of scope; the caller passes the key.

mod error;
mod list;
mod reference;
mod status;
mod token;

#[cfg(test)]
mod draft_vectors_tests;
#[cfg(test)]
mod list_tests;
#[cfg(test)]
mod reference_tests;
#[cfg(test)]
mod status_tests;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod token_tests;

pub use error::TokenStatusListError;
pub use list::{MAX_DECODED_BYTES, TokenStatusList};
pub use reference::StatusReference;
pub use status::Status;
pub use token::StatusListToken;

/// The `typ` of a Status List Token, draft-21 section 5.1: *"The JWT type MUST be
/// `statuslist+jwt`."* Its media type is `application/statuslist+jwt`.
pub const TYP: &str = "statuslist+jwt";
