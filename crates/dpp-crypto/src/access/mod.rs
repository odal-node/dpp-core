//! Access control — sector access policies, VC issuance/verification, and status lists.

pub mod credential;
pub mod filter;
pub mod policy;
pub mod status_list;
#[cfg(test)]
mod tests;

pub use credential::{
    AccessTier, AllowAllIssuers, CredentialBuilder, CredentialRole, CredentialStatus,
    DppAccessCredential, DppCredentialSubject, RevocationOutcome, StaticTrustedIssuers,
    TrustedIssuerRegistry, VerificationResult, check_revocation, verify_credential_claims,
    verify_credential_claims_with_trust, verify_credential_with_revocation,
    verify_credential_with_revocation_and_trust,
};
pub use filter::{PolicyDecision, filter_by_access_tier};
pub use policy::SectorAccessPolicy;
pub use status_list::StatusList;
