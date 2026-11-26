use chrono::{DateTime, Utc};

use super::revocation::{RevocationOutcome, check_revocation};
use super::trust::TrustedIssuerRegistry;
use super::types::{Audience, CredentialRole, DppAccessCredential};
use crate::status_list::StatusList;

// ─── Verification result ────────────────────────────────────────────────────

/// Result of verifying a DPP access credential.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    /// Credential is valid — the granted audience is returned.
    Valid {
        audience: Audience,
        role: CredentialRole,
        holder_did: String,
    },
    /// Credential has expired.
    Expired { expired_at: DateTime<Utc> },
    /// The credential's `validFrom` is later than `now`; the mirror of `Expired`.
    NotYetValid { valid_from: DateTime<Utc> },
    /// JWS signature is invalid or cannot be verified.
    InvalidSignature(String),
    /// The status bit is set, so the issuer has revoked the credential.
    Revoked,
    /// The credential declares a revocation status that could not be answered:
    /// no status list was supplied, or the list supplied cannot answer for its
    /// entry (the entry's `statusPurpose` is not `revocation`, or its index is
    /// missing, not a number, or out of range). It is not valid: a caller
    /// refuses it exactly as it refuses `Revoked`, and can now tell an
    /// unavailable list from a revocation.
    RevocationIndeterminate,
    /// Credential is structurally invalid (missing fields, wrong type).
    MalformedCredential(String),
    /// The credential's scope doesn't cover the requested resource.
    OutOfScope { reason: String },
    /// The credential's issuer DID is not in the operator's trust registry for
    /// the audience it claims to grant.
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
    required_product_group: Option<&str>,
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
        return VerificationResult::NotYetValid {
            valid_from: credential.valid_from,
        };
    }

    if let Some(product_group) = required_product_group {
        let subjects_product_groups = &credential.credential_subject.product_groups;
        if !subjects_product_groups.is_empty()
            && !subjects_product_groups.iter().any(|s| s == product_group)
        {
            return VerificationResult::OutOfScope {
                reason: format!(
                    "Credential covers product_groups {:?}, but '{}' was requested",
                    subjects_product_groups, product_group
                ),
            };
        }
    }

    let role = credential.credential_subject.role.clone();
    let audience = role.audience();
    VerificationResult::Valid {
        audience,
        role,
        holder_did: credential.credential_subject.id.clone(),
    }
}

/// Full credential verification **including revocation**, with a fail-closed
/// policy.
///
/// `status_list` is the result of fetching the credential's status list:
/// `Some(list)` when fetched and verified, `None` when there is nothing to
/// fetch **or** the fetch failed.
///
/// **Fail-closed:** a credential that *declares* a revocation status whose list
/// is unavailable, or cannot answer for its entry, is `RevocationIndeterminate`,
/// which is not valid. Only a set status bit is `Revoked`.
pub fn verify_credential_with_revocation(
    credential: &DppAccessCredential,
    required_product_group: Option<&str>,
    now: DateTime<Utc>,
    status_list: Option<&StatusList>,
) -> VerificationResult {
    let base = verify_credential_claims(credential, required_product_group, now);
    if !base.is_valid() {
        return base;
    }
    apply_revocation_check(base, credential, status_list)
}

/// Apply the fail-closed revocation check on top of an already-`Valid` `base`
/// result: no declared status → pass `base` through unchanged; a declared
/// status with no list, or a list that cannot answer for its entry →
/// `RevocationIndeterminate`; a set bit → `Revoked`. Shared by
/// [`verify_credential_with_revocation`] and
/// [`verify_credential_with_revocation_and_trust`], which differ only in the
/// claims check performed before this point.
fn apply_revocation_check(
    base: VerificationResult,
    credential: &DppAccessCredential,
    status_list: Option<&StatusList>,
) -> VerificationResult {
    if credential.credential_status.is_none() {
        return base;
    }
    match status_list {
        None => VerificationResult::RevocationIndeterminate,
        Some(list) => match check_revocation(credential, list) {
            RevocationOutcome::NotRevoked => base,
            RevocationOutcome::Revoked => VerificationResult::Revoked,
            RevocationOutcome::Indeterminate => VerificationResult::RevocationIndeterminate,
        },
    }
}

/// Verify structural validity, scope, and **issuer trust** of a credential
/// (no signature check — that is the JWS verifier's responsibility).
pub fn verify_credential_claims_with_trust(
    credential: &DppAccessCredential,
    required_product_group: Option<&str>,
    required_product_category: Option<&str>,
    now: DateTime<Utc>,
    trusted_issuers: &dyn TrustedIssuerRegistry,
) -> VerificationResult {
    let base = verify_credential_claims(credential, required_product_group, now);
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

    let required_audience = credential.credential_subject.role.audience();
    if !trusted_issuers.is_trusted_for_audience(&credential.issuer, required_audience) {
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
    required_product_group: Option<&str>,
    required_product_category: Option<&str>,
    now: DateTime<Utc>,
    status_list: Option<&StatusList>,
    trusted_issuers: &dyn TrustedIssuerRegistry,
) -> VerificationResult {
    let base = verify_credential_claims_with_trust(
        credential,
        required_product_group,
        required_product_category,
        now,
        trusted_issuers,
    );
    if !base.is_valid() {
        return base;
    }
    apply_revocation_check(base, credential, status_list)
}
