use crate::access::status_list::StatusList;

use super::types::DppAccessCredential;

/// Outcome of resolving a credential's revocation status against a status list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RevocationOutcome {
    /// The status bit is clear — the credential is not revoked.
    NotRevoked,
    /// The status bit is set — the credential is revoked.
    Revoked,
    /// The credential declares a status that the provided list cannot answer
    /// (no/invalid index, index out of range). Callers MUST fail closed.
    Indeterminate,
}

/// Resolve a credential's revocation status against an **already-fetched** status
/// list. Fetching the status-list credential over the network is an
/// infrastructure concern handled by the platform (crypto Gap 5) — this is the
/// pure decision given the list.
///
/// A credential that declares no `credentialStatus` is `NotRevoked` (there is
/// nothing to revoke against).
pub fn check_revocation(
    credential: &DppAccessCredential,
    status_list: &StatusList,
) -> RevocationOutcome {
    let Some(status) = credential.credential_status.as_ref() else {
        return RevocationOutcome::NotRevoked;
    };
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
