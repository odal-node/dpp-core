//! W3C Verifiable Credentials for DPP access control.
//!
//! This module implements VC issuance and verification following the
//! W3C Verifiable Credentials Data Model v2.0 specification, adapted
//! for EU Digital Product Passport access tiers.
//!
//! ## Access tiers
//!
//! The ESPR mandates three tiers of data access:
//! - **Public**: No credential required.
//! - **Professional**: Requires a VC proving the holder's role (repairer, recycler, etc.).
//! - **Confidential**: Requires an institutional DID (market surveillance authority).
//!
//! ## Credential lifecycle
//!
//! 1. An authority issues a `DppAccessCredential` to an operator.
//! 2. The credential is signed as a JWS using the issuer's Ed25519 key.
//! 3. When requesting professional/confidential data, the holder presents the VC.
//! 4. The verifier checks the JWS, expiration, revocation status, and scope.

mod builder;
mod revocation;
#[cfg(test)]
mod tests;
mod trust;
mod types;
mod verify;

pub use builder::CredentialBuilder;
pub use revocation::{RevocationOutcome, check_revocation};
pub use trust::{AllowAllIssuers, StaticTrustedIssuers, TrustedIssuerRegistry};
pub use types::{
    AccessTier, CredentialRole, CredentialStatus, DppAccessCredential, DppCredentialSubject,
};
pub use verify::{
    VerificationResult, verify_credential_claims, verify_credential_claims_with_trust,
    verify_credential_with_revocation, verify_credential_with_revocation_and_trust,
};
