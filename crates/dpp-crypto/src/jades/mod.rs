//! JAdES baseline signatures — ETSI TS 119 182-1 V1.2.1 (2024-07).
//!
//! JAdES is an AdES signature format built directly on JWS (IETF RFC 7515), and
//! its scope covers **qualified electronic seals** as defined in Regulation (EU)
//! No 910/2014. That is the fact this module exists to exploit: a qualified seal
//! does not have to be a document-format container over bytes. It can be a JWS.
//!
//! # Why this is here rather than in an adapter
//!
//! A remote signing provider signs bytes. Everything else about a JAdES
//! signature — which header parameters are present, how the signing input is
//! derived, how the compact form is assembled — is format work, and format work
//! is a primitive.
//!
//! Splitting it this way is what makes the sealing provider replaceable. An
//! adapter that asked its provider to *produce* a JAdES would only work with
//! providers that sell JAdES; one that asks for a raw signature over a
//! caller-supplied digest works with essentially all of them, because
//! signing a hash is the one operation every provider offers. So the format
//! stays here, the network stays in the platform, and which provider signs is a
//! deployment question.
//!
//! # What this module does not claim
//!
//! **It does not make a signature qualified.** Qualified status comes from the
//! certificate, the creation device and the trust service provider behind them —
//! Art. 3(27) of Regulation (EU) No 910/2014 makes it a three-part conjunction,
//! and none of the three is a property of the bytes assembled here. This module
//! produces a *structurally conformant JAdES-B-B signature*. What that signature
//! is worth depends entirely on whose key signed it.
//!
//! It also does not validate. Validation is ETSI EN 319 102-1 and needs trust
//! anchors, certificate paths and revocation data — a different and much larger
//! object. See [`crate::jws::verifier`] for the plain-JWS signature check, which
//! is a far smaller claim.
//!
//! # Scope: B-B, attached payload
//!
//! Only the baseline B-B level with an attached payload is implemented, which is
//! deliberate and is the level that composes best:
//!
//! - **B-B** is the signature alone. Higher levels need a timestamp authority
//!   (`B-T`), revocation and certificate material (`B-LT`), or archival
//!   timestamps (`B-LTA`) — each an external service, none of them format work.
//!   `SealConformanceLevel` in `dpp-domain` models all four so a caller can
//!   *ask* for more; this module implements the one that needs nothing but a
//!   signature.
//! - **Attached** means the payload travels inside the JWS. Detached payloads
//!   use the `sigD` header parameter (TS 119 182-1 clause 5.2.8), which then
//!   forces `crit` to be present. Keeping the payload attached avoids both, and
//!   the result is a signature a plain RFC 7515 library can still parse — see
//!   [`header`] for why the specification now goes out of its way to allow that.
//!
//! # Reading order
//!
//! - [`header`] — the protected header, and which parameters the standard
//!   actually requires.
//! - [`builder`] — the two-phase construction a remote signer needs.

pub mod builder;
pub mod header;

pub use builder::{JadesSignature, PreparedJades, prepare};
pub use header::{CertificateRef, JadesError, JadesHeader};

#[cfg(test)]
mod tests;
