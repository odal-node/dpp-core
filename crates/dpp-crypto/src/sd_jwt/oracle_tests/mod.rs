//! The SD-JWT corpus an independent implementation judges, and what it issues
//! back.
//!
//! The Rust side of `.github/workflows/sdjwt-oracle.yml`. Every other test in
//! this module family holds `dpp_crypto::sd_jwt` to this repository's own reading
//! of RFC 9901, or to the RFC's examples, which fix their salts and cannot show
//! how an issuer builds the payload around the digests. An implementation with
//! no code and no author in common with this one is the only check that is not
//! circular, so this module produces tokens for one and reads tokens from one.
//!
//! - [`plan`] issues a claim tree to a plan, and works out independently what a
//!   verifier must see when only some of it is presented.
//! - [`issued`] is the corpus of tokens this crate issues.
//! - [`adversarial`] is the corpus of tokens it should refuse, each with the
//!   verdict this crate gives, so the two implementations can be compared.
//! - [`rfc`] is the RFC's own tokens, as vendored in this repository.
//! - [`emit`] writes them out. [`reverse`] reads back what the other
//!   implementation issued.
//!
//! Without `EMIT_SDJWT_CORPUS` the tests here still run and still check that the
//! corpus is complete and that this crate agrees with the plan, so an ordinary
//! `just check` gets that coverage without producing files.

mod adversarial;
mod emit;
mod issued;
mod plan;
mod reverse;
mod rfc;
mod signing;
