//! `dpp-crypto` — Ed25519 signing, JWS compact serialisation, DID documents,
//! and access-tier Verifiable Credentials.
//!
//! All modules are pure (no I/O, no network). The crate compiles to `std` for
//! the node and to `wasm32` (where conditional) for plugin guests.

pub mod access;
pub mod identity;
pub mod jws;
pub mod keystore;

// ── Flat re-exports — maintain stable paths for external callers ─────────────

pub use access::{
    AccessTier, AllowAllIssuers, CredentialBuilder, CredentialRole, CredentialStatus,
    DppAccessCredential, DppCredentialSubject, PolicyDecision, RevocationOutcome,
    SectorAccessPolicy, StaticTrustedIssuers, StatusList, TrustedIssuerRegistry,
    VerificationResult, check_revocation, filter_by_access_tier, verify_credential_claims,
    verify_credential_claims_with_trust, verify_credential_with_revocation,
    verify_credential_with_revocation_and_trust,
};
pub use identity::{LocalIdentityService, PassportCredential, PassportCredentialSubject};
