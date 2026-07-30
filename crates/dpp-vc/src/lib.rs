//! `dpp-vc` — the trust layer: who may read a passport, and how that is proven.
//!
//! W3C Verifiable Credentials, `did:web` documents, Bitstring Status List
//! revocation, and the JSON-LD context those are expressed in.
//!
//! Pure (no I/O, no network). Not a `wasm32-unknown-unknown` target: it depends
//! on [`dpp_crypto`], whose RNG requires a platform entropy source.
//!
//! # Trust, not primitives, and not policy
//!
//! Three neighbouring concerns are deliberately **not** here:
//!
//! - **Cryptographic primitives** — JWS, keys, the keystore — are
//!   [`dpp_crypto`], which this crate depends on. Signing bytes is a different
//!   job from deciding whose signature means what.
//! - **The disclosure contract** — `Audience`, `Disclosure`, the per-field
//!   disclosure map and the filter that applies it — describes what a passport
//!   *is*, not who is asking. It currently lives in `dpp-crypto::access`; its
//!   settled home is `dpp-domain`.
//! - **Projections** — GS1, AAS — render a passport for an ecosystem. They say
//!   nothing about trust.
//!
//! The seam this crate sits on: **a credential establishes which `Audience` a
//! caller holds; the disclosure policy maps that `Audience` to fields.** Those
//! are two questions and they are answered in two places.

pub mod credential;
pub mod did_builder;
pub mod jsonld;
pub mod local_service;
pub mod passport_credential;
pub mod status_list;

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;

pub use credential::{
    AllowAllIssuers, Audience, CredentialBuilder, CredentialRole, CredentialStatus,
    DppAccessCredential, DppCredentialSubject, RevocationOutcome, StaticTrustedIssuers,
    TrustedIssuerRegistry, VerificationResult, check_revocation, verify_credential_claims,
    verify_credential_claims_with_trust, verify_credential_with_revocation,
    verify_credential_with_revocation_and_trust,
};
pub use did_builder::build_did_document;
pub use jsonld::{frame_passport, passport_context, strip_context};
pub use local_service::LocalIdentityService;
pub use passport_credential::{PassportCredential, PassportCredentialSubject};
pub use status_list::StatusList;

/// Compile-checks this crate's README examples.
///
/// A README example is a public claim about the API, and nothing else in the
/// build compiles one. Without this, a README can advertise a function that
/// does not exist — which is exactly what happened before this harness landed.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct ReadmeDoctests;
