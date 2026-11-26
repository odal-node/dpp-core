//! Passport verifiable-credential construction.
//!
//! `PassportCredential`/`PassportCredentialSubject` are defined in `dpp-domain`
//! and re-exported here for callers that import from `dpp-crypto`. This module
//! additionally owns the signing-layer convention for turning a `PassportId`
//! into a VC subject id: `dpp-domain` stays URN-agnostic, so the `urn:uuid:`
//! formatting lives on the crypto side, next to the credential it's stamped into.

mod build;
#[cfg(test)]
mod tests;

pub use build::*;
