//! SD-JWT VC — a passport as a credential a holder can present in part.
//!
//! Implements **IETF RFC 9901** (Selective Disclosure for JSON Web Tokens,
//! Standards Track, November 2025) through [`dpp_crypto::sd_jwt`], and the
//! credential profile **draft-ietf-oauth-sd-jwt-vc-19** (31 August 2026), read
//! on 2026-09-16.
//!
//! **No conformance is claimed.** The profile is an Internet-Draft — submitted
//! to the IESG, but not published — and an Internet-Draft is explicitly not
//! reference material. What can honestly be said is the sentence above: which
//! documents were read, and when.
//!
//! # What this is for
//!
//! The existing read path filters server-side: a caller presents a credential,
//! the node decides what that audience may see, and returns the remainder. That
//! works, and it puts the node in the trust path of every read.
//!
//! This is the other shape. The issuer signs once over salted digests and hands
//! the cleartext to the holder as disclosures. The holder forwards the signed
//! token plus whichever disclosures it chooses, and a third party verifies the
//! subset without the issuing node being reachable at all.
//!
//! It does not replace the audience-scoped read. Both exist: the endpoint for
//! callers who want the node to filter, the credential for holders who want to
//! prove things without it.
//!
//! # The property that is hard to get any other way
//!
//! A passport's signed views are frozen at publish, and publishing is a one-way
//! transition. So a field reclassified from public to restricted *after* publish
//! cannot be withdrawn from a passport already issued — its cleartext is inside
//! a signature that cannot be remade. Under selective disclosure, withdrawal is
//! free: stop releasing that disclosure and the signature still verifies.
//!
//! That is why [`issue`] conceals **every non-public claim** rather than only
//! the ones some audience is currently denied. A claim issued in cleartext can
//! never be withdrawn, so anything whose classification could move must be
//! disclosable from the start.
//!
//! The limit, stated plainly: this constrains a party that has not already been
//! given the disclosure. A holder who has it can still reveal it.
//!
//! # What is deliberately absent
//!
//! - **Key binding.** RFC 9901 clause 4.3 proves the presenter holds a key the
//!   credential names, which is real security and requires holders to have keys.
//!   That is an ecosystem assumption, not a code change, so the design leaves
//!   room for it and ships without it.
//! - **`x5c` issuance.** Clause 2.5's other key-discovery mechanism makes the
//!   issuer the subject of an end-entity certificate. See
//!   [`issuer_metadata::build_issuer_metadata`] for why that is not this
//!   module's decision to make.
//! - **Type Metadata.** Clause 5 lets a `vct` publish per-claim `sd` and
//!   `mandatory` rules that a verifier then enforces. That maps closely onto the
//!   per-property disclosure classes the schemas already carry, and is worth
//!   doing — but it is a second artefact to publish and version, and it is not
//!   needed for a credential to verify.
//! - **Replacing the per-audience signatures.** Those stay. Cutting them over is
//!   a wire change to every passport, and nothing outside this repository
//!   consumes an SD-JWT VC yet.

pub mod error;
mod issue;
pub mod issuer_metadata;
mod vct;
mod verify;

#[cfg(test)]
mod issuer_metadata_tests;
#[cfg(test)]
mod test_fixtures;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod validity_tests;
#[cfg(test)]
mod vct_tests;

pub use error::SdJwtVcError;
pub use issue::issue;
pub use issuer_metadata::{WELL_KNOWN_PATH, build_issuer_metadata};
pub use vct::vct_for;
pub use verify::verify;

/// The media type of the Issuer-signed JWT, draft-19 clause 2.2.1.
///
/// `dc+sd-jwt`, not `vc+sd-jwt`: the transitional allowance for the older value
/// was **removed** in -19, so emitting it would now be wrong rather than merely
/// dated.
pub const TYP: &str = "dc+sd-jwt";
