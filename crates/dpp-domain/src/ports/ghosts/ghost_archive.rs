//! [`GhostArchive`] — a no-op archive for development and standalone vaults.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::domain::error::DppError;
use crate::domain::passport::{Passport, PassportId};
use crate::ports::archive::{
    ArchivePort, ArchiveReceipt, ArchiveStatus, ArchiveVerification, retention_deadline,
};

/// No-op archive for development and standalone vault deployments.
///
/// All operations succeed without performing any I/O. Returns synthetic
/// receipts with `archive_id = "ghost-{uuid}"`. Use in tests and in the
/// standalone `dpp-vault` binary where object storage is not configured.
pub struct GhostArchive;

#[async_trait]
impl ArchivePort for GhostArchive {
    async fn archive(
        &self,
        passport: &Passport,
        retention_years: u32,
    ) -> Result<ArchiveReceipt, DppError> {
        let now = Utc::now();
        Ok(ArchiveReceipt {
            archive_id: format!("GHOST-{}", Uuid::now_v7()),
            passport_id: passport.id,
            content_hash: String::new(),
            archived_at: now,
            retention_until: retention_deadline(now, retention_years),
        })
    }

    async fn update_archive(&self, passport: &Passport) -> Result<ArchiveReceipt, DppError> {
        let now = Utc::now();
        Ok(ArchiveReceipt {
            archive_id: format!("GHOST-{}", Uuid::now_v7()),
            passport_id: passport.id,
            content_hash: String::new(),
            archived_at: now,
            // `update_archive` has no `retention_years` parameter (see
            // `ArchivePort` trait) so the general 10-year default is the best
            // this ghost can do without tracking state from the original
            // `archive` call.
            retention_until: retention_deadline(now, 10),
        })
    }

    async fn verify(
        &self,
        _passport_id: PassportId,
        _expected_hash: &str,
    ) -> Result<ArchiveVerification, DppError> {
        Ok(ArchiveVerification {
            integrity_ok: false,
            accessible: false,
            status: ArchiveStatus::Expired,
            last_verified_at: Utc::now(),
        })
    }

    async fn retrieve(&self, _passport_id: PassportId) -> Result<Option<Passport>, DppError> {
        Ok(None)
    }
}
