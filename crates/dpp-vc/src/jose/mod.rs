//! VC-JOSE-COSE: any W3C Verifiable Credential or Verifiable Presentation, secured
//! as a compact JWS.
//!
//! Implements **Securing Verifiable Credentials using JOSE and COSE**, W3C
//! Recommendation of 15 May 2025 (`REC-vc-jose-cose-20250515`), for the JOSE half
//! only: a credential is a `vc+jwt` (§3.1.1) and a presentation is a `vp+jwt`
//! (§3.1.2), each a compact JWS (RFC 7515) whose payload is the credential or
//! presentation itself. The specification's SD-JWT (§3.2) and COSE (§3.3) secured
//! forms are not implemented here. [`crate::sd_jwt_vc`] is a different format, the
//! IETF SD-JWT VC profile.
//!
//! **No conformance is claimed.** What was read and what each part is checked
//! against is recorded in the standards register. A claim of conformance waits for
//! the W3C test suite's results.
//!
//! # How this differs from the access credential
//!
//! [`crate::credential::jwt`] signs one typed document, a [`DppAccessCredential`],
//! and verifies it through an issuer's `did:web` document. This module signs and
//! verifies **any** credential or presentation, given as JSON, under a key the
//! caller supplies. It resolves nothing and performs no I/O: a token's `kid` is a
//! hint (§4.1.1), and choosing the key is the caller's, the way
//! [`sd_jwt_vc::verify`](crate::sd_jwt_vc::verify) leaves it.
//!
//! [`DppAccessCredential`]: crate::credential::DppAccessCredential
//!
//! # Signing
//!
//! [`sign_credential`] and [`sign_presentation`] take the document, a
//! [`KeyEntry`](dpp_crypto::keystore::KeyEntry), the `kid`, and the extension terms
//! the caller means to use. The header carries `alg`, `kid`, `typ` and `cty`, and
//! the payload is the document unchanged in RFC 8785 canonical form. A key from
//! [`KeyEntry::from_private_jwk`](dpp_crypto::keystore::KeyEntry::from_private_jwk)
//! signs as well as one a key store generated, under EdDSA, ES256, ES384 or ES512.
//!
//! **Undeclared members are refused at signing.** A top-level member must be a term
//! of the VC Data Model 2.0 base vocabulary ([`CREDENTIAL_TERMS`] or
//! [`PRESENTATION_TERMS`]), a JWT registered claim name of RFC 7519 §4.1, or named
//! in `extension_terms`. `vc` and `vp` are refused even when named (§1.1.2.1).
//! Verification ignores members it does not understand, as §3.1.3 and §5.4 require:
//! the discipline is an issuer's, not a verifier's.
//!
//! Signing also refuses what verification would refuse for the same document: an `iss`
//! that is not the issuer, and an `exp` or `nbf` that is not a JSON number. No time is
//! compared at signing.
//!
//! # Verifying a credential
//!
//! [`verify_credential`] takes the issuer's [`PublicKey`](dpp_crypto::jws::PublicKey)
//! and refuses at the first failed check, in this order: compact form, protected
//! header (readable, no `crit`, no `b64` of `false`, `alg` is the key's, so `alg:
//! none` is refused), signature, `typ` and `cty`, payload and its `@context`, `vc`
//! and `vp`, time, the issuer binding of §4.1.2 and the `kid` of §4.2, then the
//! credential's `type`, `issuer` and `credentialSubject`. Every refusal is a variant
//! of [`JoseError`].
//!
//! **Time is explicit.** `now: Option<i64>`, seconds since the epoch. Under
//! `Some(now)`, `exp` must be a number and `now` before it, and `nbf` must be a
//! number and not after `now`, with no leeway, as in `sd_jwt_vc::verify`. Under
//! `None` neither claim is read. `iat` is never checked: §3.1.3 says it describes
//! the signature, not the credential. A credential's own `validFrom` and
//! `validUntil` are the caller's to check.
//!
//! # Verifying a presentation
//!
//! Verifying a presentation and verifying the credentials it envelops are two
//! operations. VC-JOSE-COSE §5.1 defines verifying a presentation secured with JOSE as
//! validating its own JWT (RFC 7519), and §5 says key discovery is assumed to have
//! succeeded before each document is verified. So [`verify_presentation`] checks the
//! presentation's outer token completely, then the shape of every envelope (§3.1.2),
//! and returns a [`VerifiedPresentation`] whose envelopes are **not** verified. Each
//! credential is verified afterwards, under the key the caller has for its issuer: one at
//! a time with [`UnverifiedEnvelope::verify`], or all of them with
//! [`VerifiedPresentation::verify_credentials`], which fails if any one fails. Only a
//! `vc+jwt` can be verified, so an envelope of another format makes
//! `verify_credentials` fail with `UnsupportedEnvelopeFormat`. §5.4 asks for further
//! validation once verification succeeds; the checks this crate does not make are listed
//! below.
//!
//! # What is deliberately absent
//!
//! - **JSON-LD expansion.** `@context` is checked against the VC Data Model 2.0 context
//!   set (§4.3), and nothing is fetched or expanded: this crate does no network access.
//! - **Key discovery.** §4.2 leaves how a verifier finds a key to the verifier, and
//!   this crate does not check that a `kid` names a method the issuer controls.
//! - **`credentialStatus` and `credentialSchema` checks** (§5.4 SHOULD), and the
//!   credential's validity period.
//! - **`crit`.** Any `crit` is refused, which is stricter than RFC 7515 §4.1.11,
//!   since no critical extension is understood.

mod absolute_url;
mod envelope_shape;
pub mod error;
mod presentation;
mod properties;
mod sign;
mod unverified_envelope;
mod verified_presentation;
mod verify;
mod vocabulary;

#[cfg(test)]
mod algorithm_tests;
#[cfg(test)]
mod credentials_tests;
#[cfg(test)]
mod envelope_shape_tests;
#[cfg(test)]
mod fixtures_tests;
#[cfg(test)]
mod issuer_tests;
#[cfg(test)]
mod presentation_tests;
#[cfg(test)]
mod properties_tests;
#[cfg(test)]
mod sign_tests;
#[cfg(test)]
mod suite_issue_tests;
#[cfg(test)]
mod suite_tests;
#[cfg(test)]
mod temporal_tests;
#[cfg(test)]
mod verify_tests;
#[cfg(test)]
mod vocabulary_tests;

pub use error::JoseError;
pub use presentation::verify_presentation;
pub use sign::{sign_credential, sign_presentation};
pub use unverified_envelope::UnverifiedEnvelope;
pub use verified_presentation::VerifiedPresentation;
pub use verify::verify_credential;
pub use vocabulary::{CREDENTIAL_TERMS, PRESENTATION_TERMS};
