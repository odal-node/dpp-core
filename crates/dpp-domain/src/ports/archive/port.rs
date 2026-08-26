//! [`ArchivePort`] — the contract a third-party archive adapter implements.

use async_trait::async_trait;

use super::receipt::ArchiveReceipt;
use super::verification::ArchiveVerification;
use crate::error::DppError;
use crate::passport::{Passport, PassportId};

/// Port trait for replicating DPP records to an independent third-party archive.
///
/// Called automatically when a passport is published. Platform adapters
/// implement this trait to connect to the chosen archive service provider.
///
/// # SLA expectations
///
/// The archive provider MUST:
/// - Accept and store the record within the SLA window (recommended < 30s).
/// - Return a content hash for integrity verification.
/// - Retain the record for the full retention period.
/// - Serve the record upon authenticated request even if the originating
///   operator's infrastructure is unreachable (insolvency failover).
#[async_trait]
pub trait ArchivePort: Send + Sync {
    /// Archive a published passport.
    ///
    /// Called on the `Draft → Published` transition. The passport's JWS
    /// signature MUST be present (i.e. the passport has been signed).
    ///
    /// `retention_years` is derived from the applicable delegated act
    /// (typically 10–15 years after the product's end of life).
    async fn archive(
        &self,
        passport: &Passport,
        retention_years: u32,
    ) -> Result<ArchiveReceipt, DppError>;

    /// Update an existing archived record.
    ///
    /// Called when a passport is updated after a transfer of responsibility
    /// or when compliance data is corrected. The archive MUST store the
    /// new version while preserving the full version history.
    async fn update_archive(&self, passport: &Passport) -> Result<ArchiveReceipt, DppError>;

    /// Verify that the archive holds an intact copy of the passport.
    ///
    /// Compares a content hash against the archived payload. Used for
    /// periodic integrity audits and compliance verification.
    async fn verify(
        &self,
        passport_id: PassportId,
        expected_hash: &str,
    ) -> Result<ArchiveVerification, DppError>;

    /// Retrieve a passport from the archive.
    ///
    /// Used as a failover when the originating operator's infrastructure
    /// is unreachable. Returns `None` if the archive has no record.
    async fn retrieve(&self, passport_id: PassportId) -> Result<Option<Passport>, DppError>;
}
