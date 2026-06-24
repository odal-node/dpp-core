use chrono::{DateTime, Utc};

use super::revocation::{RevocationOutcome, check_revocation};
use super::trust::TrustedIssuerRegistry;
use super::types::{AccessTier, CredentialRole, DppAccessCredential};
use crate::access::status_list::StatusList;

// ─── Verification result ────────────────────────────────────────────────────

/// Result of verifying a DPP access credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// Credential is valid — the granted access tier is returned.
    Valid {
        access_tier: AccessTier,
        role: CredentialRole,
        holder_did: String,
    },
    /// Credential has expired.
    Expired { expired_at: DateTime<Utc> },
    /// JWS signature is invalid or cannot be verified.
    InvalidSignature(String),
    /// Credential has been revoked.
    Revoked,
    /// Credential is structurally invalid (missing fields, wrong type).
    MalformedCredential(String),
    /// The credential's scope doesn't cover the requested resource.
    OutOfScope { reason: String },
    /// The credential's issuer DID is not in the operator's trust registry for
    /// the tier it claims to grant (Gap 9: issuer trust anchor).
    UntrustedIssuer { issuer_did: String },
}

impl VerificationResult {
    pub fn is_valid(&self) -> bool {
        matches!(self, Self::Valid { .. })
    }
}

// ─── Verify functions ────────────────────────────────────────────────────────

/// Verify structural validity and expiration of a credential (no signature check).
///
/// Signature verification requires the issuer's public key and is done
/// separately via the JWS verifier. This function handles the credential-
/// level checks: type, expiration, and scope.
pub fn verify_credential_claims(
    credential: &DppAccessCredential,
    required_sector: Option<&str>,
    now: DateTime<Utc>,
) -> VerificationResult {
    if !credential
        .credential_type
        .contains(&"VerifiableCredential".to_owned())
    {
        return VerificationResult::MalformedCredential(
            "Missing 'VerifiableCredential' type".into(),
        );
    }

    if now > credential.valid_until {
        return VerificationResult::Expired {
            expired_at: credential.valid_until,
        };
    }

    if now < credential.valid_from {
        return VerificationResult::MalformedCredential(
            "Credential issuance date is in the future".into(),
        );
    }

    if let Some(sector) = required_sector {
        let subjects_sectors = &credential.credential_subject.sectors;
        if !subjects_sectors.is_empty() && !subjects_sectors.iter().any(|s| s == sector) {
            return VerificationResult::OutOfScope {
                reason: format!(
                    "Credential covers sectors {:?}, but '{}' was requested",
                    subjects_sectors, sector
                ),
            };
        }
    }

    let role = credential.credential_subject.role.clone();
    let access_tier = role.access_tier();
    VerificationResult::Valid {
        access_tier,
        role,
        holder_did: credential.credential_subject.id.clone(),
    }
}

/// Full credential verification **including revocation**, with a fail-closed
/// policy (crypto Gap 5).
///
/// `status_list` is the result of fetching the credential's status list:
/// `Some(list)` when fetched and verified, `None` when there is nothing to
/// fetch **or** the fetch failed.
///
/// **Fail-closed:** a credential that *declares* a revocation status whose list
/// is unavailable or unresolvable is treated as `Revoked`.
pub fn verify_credential_with_revocation(
    credential: &DppAccessCredential,
    required_sector: Option<&str>,
    now: DateTime<Utc>,
    status_list: Option<&StatusList>,
) -> VerificationResult {
    let base = verify_credential_claims(credential, required_sector, now);
    if !base.is_valid() {
        return base;
    }
    if credential.credential_status.is_none() {
        return base;
    }
    match status_list {
        None => VerificationResult::Revoked,
        Some(list) => match check_revocation(credential, list) {
            RevocationOutcome::NotRevoked => base,
            RevocationOutcome::Revoked | RevocationOutcome::Indeterminate => {
                VerificationResult::Revoked
            }
        },
    }
}

/// Verify structural validity, scope, and **issuer trust** of a credential
/// (no signature check — that is the JWS verifier's responsibility).
pub fn verify_credential_claims_with_trust(
    credential: &DppAccessCredential,
    required_sector: Option<&str>,
    required_product_category: Option<&str>,
    now: DateTime<Utc>,
    trusted_issuers: &dyn TrustedIssuerRegistry,
) -> VerificationResult {
    let base = verify_credential_claims(credential, required_sector, now);
    if !base.is_valid() {
        return base;
    }

    if let Some(required_cat) = required_product_category {
        let cats = &credential.credential_subject.product_categories;
        if !cats.is_empty() && !cats.iter().any(|c| c == required_cat) {
            return VerificationResult::OutOfScope {
                reason: format!(
                    "Credential covers product categories {:?}, but '{}' was requested",
                    cats, required_cat
                ),
            };
        }
    }

    let required_tier = credential.credential_subject.role.access_tier();
    if !trusted_issuers.is_trusted_for_tier(&credential.issuer, required_tier) {
        return VerificationResult::UntrustedIssuer {
            issuer_did: credential.issuer.clone(),
        };
    }

    base
}

/// Full credential verification including **revocation** and **issuer trust**,
/// with a fail-closed policy.
pub fn verify_credential_with_revocation_and_trust(
    credential: &DppAccessCredential,
    required_sector: Option<&str>,
    required_product_category: Option<&str>,
    now: DateTime<Utc>,
    status_list: Option<&StatusList>,
    trusted_issuers: &dyn TrustedIssuerRegistry,
) -> VerificationResult {
    let base = verify_credential_claims_with_trust(
        credential,
        required_sector,
        required_product_category,
        now,
        trusted_issuers,
    );
    if !base.is_valid() {
        return base;
    }
    if credential.credential_status.is_none() {
        return base;
    }
    match status_list {
        None => VerificationResult::Revoked,
        Some(list) => match check_revocation(credential, list) {
            RevocationOutcome::NotRevoked => base,
            RevocationOutcome::Revoked | RevocationOutcome::Indeterminate => {
                VerificationResult::Revoked
            }
        },
    }
}
