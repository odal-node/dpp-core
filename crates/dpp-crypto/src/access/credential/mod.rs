//! W3C Verifiable Credentials for DPP access control.
//!
//! This module implements VC issuance and verification following the
//! W3C Verifiable Credentials Data Model v2.0 specification, adapted
//! to the EU Digital Product Passport audience model.
//!
//! ## Audiences
//!
//! Reg. (EU) 2023/1542 Art. 77(2) names three audiences, and the credential
//! establishes which one a caller belongs to:
//! - **Public**: no credential required.
//! - **LegitimateInterest**: a VC proving the holder's role (repairer,
//!   remanufacturer, second-life operator, recycler).
//! - **Authority**: an institutional DID (notified body, market surveillance
//!   authority, customs, the Commission).
//!
//! These do not form a ranking. An `Authority` sees conformity test reports
//! that a `LegitimateInterest` holder does not, and a `LegitimateInterest`
//! holder sees individual-item data that an `Authority` does not. See
//! [`Audience::may_see`](dpp_domain::Audience::may_see).
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
    Audience, CredentialRole, CredentialStatus, DppAccessCredential, DppCredentialSubject,
};
pub use verify::{
    VerificationResult, verify_credential_claims, verify_credential_claims_with_trust,
    verify_credential_with_revocation, verify_credential_with_revocation_and_trust,
};
