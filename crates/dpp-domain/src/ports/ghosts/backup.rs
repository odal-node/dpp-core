//! [`GhostBackup`] — a no-op back-up copy for development and standalone nodes.

use async_trait::async_trait;
use chrono::Utc;
use uuid::Uuid;

use crate::error::dpp::DppError;
use crate::passport::{Passport, PassportId};
use crate::ports::backup::{
    BackupCopyPort, BackupReceipt, BackupStatus, BackupVerification, retention_deadline,
};

/// No-op back-up copy for development and standalone deployments.
///
/// All operations succeed without performing any I/O. Returns synthetic
/// receipts with `backup_id = "ghost-{uuid}"`. Use in tests and in the
/// standalone `dpp-vault` binary where object storage is not configured.
pub struct GhostBackup;

#[async_trait]
impl BackupCopyPort for GhostBackup {
    async fn store(
        &self,
        passport: &Passport,
        retention_years: u32,
    ) -> Result<BackupReceipt, DppError> {
        let now = Utc::now();
        Ok(BackupReceipt {
            backup_id: format!("GHOST-{}", Uuid::now_v7()),
            passport_id: passport.id,
            content_hash: String::new(),
            stored_at: now,
            retention_until: retention_deadline(now, retention_years),
        })
    }

    async fn update(&self, passport: &Passport) -> Result<BackupReceipt, DppError> {
        let now = Utc::now();
        Ok(BackupReceipt {
            backup_id: format!("GHOST-{}", Uuid::now_v7()),
            passport_id: passport.id,
            content_hash: String::new(),
            stored_at: now,
            // `update` has no `retention_years` parameter (see
            // `BackupCopyPort` trait) so the general 10-year default is the best
            // this ghost can do without tracking state from the original
            // `store` call.
            retention_until: retention_deadline(now, 10),
        })
    }

    async fn verify(
        &self,
        _passport_id: PassportId,
        _expected_hash: &str,
    ) -> Result<BackupVerification, DppError> {
        Ok(BackupVerification {
            integrity_ok: false,
            accessible: false,
            status: BackupStatus::Expired,
            last_verified_at: Utc::now(),
        })
    }

    async fn retrieve(&self, _passport_id: PassportId) -> Result<Option<Passport>, DppError> {
        Ok(None)
    }
}
