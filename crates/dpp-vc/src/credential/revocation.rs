use crate::status_list::StatusList;

use super::types::DppAccessCredential;

/// Outcome of resolving a credential's revocation status against a status list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationOutcome {
    /// The status bit is clear — the credential is not revoked.
    NotRevoked,
    /// The status bit is set — the credential is revoked.
    Revoked,
    /// The credential declares a status that the provided list cannot answer
    /// (no/invalid index, index out of range, or an entry whose purpose is not
    /// revocation). Callers MUST fail closed.
    Indeterminate,
}

/// The `statusPurpose` an entry must carry for a revocation check to answer it.
pub const REVOCATION_PURPOSE: &str = "revocation";

/// Resolve a credential's revocation status against an **already-fetched** status
/// list. Fetching the status-list credential over the network is an
/// infrastructure concern handled by the host — this is the
/// pure decision given the list.
///
/// A credential that declares no `credentialStatus` is `NotRevoked` (there is
/// nothing to revoke against).
///
/// An entry whose `statusPurpose` is not `revocation` is `Indeterminate`. A set
/// bit in a suspension list means something else, and a clear one does not say
/// the credential is unrevoked, so this check cannot answer for it either way.
pub fn check_revocation(
    credential: &DppAccessCredential,
    status_list: &StatusList,
) -> RevocationOutcome {
    let Some(status) = credential.credential_status.as_ref() else {
        return RevocationOutcome::NotRevoked;
    };
    if status.status_purpose != REVOCATION_PURPOSE {
        return RevocationOutcome::Indeterminate;
    }
    let Some(index) = status
        .status_list_index
        .as_ref()
        .and_then(|s| s.parse::<usize>().ok())
    else {
        return RevocationOutcome::Indeterminate;
    };
    match status_list.get(index) {
        Some(true) => RevocationOutcome::Revoked,
        Some(false) => RevocationOutcome::NotRevoked,
        None => RevocationOutcome::Indeterminate,
    }
}
