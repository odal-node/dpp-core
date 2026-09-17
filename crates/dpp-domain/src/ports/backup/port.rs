//! [`BackupCopyPort`] — the contract a back-up copy provider's adapter
//! implements.

use async_trait::async_trait;

use super::receipt::BackupReceipt;
use super::verification::BackupVerification;
use crate::error::DppError;
use crate::passport::{Passport, PassportId};

/// Port trait for replicating DPP records to an independent third party.
///
/// Called automatically when a passport is published. Platform adapters
/// implement this trait to connect to the chosen passport service provider
/// (ESPR Art. 2(32)).
///
/// # SLA expectations
///
/// The provider MUST:
/// - Accept and store the record within the SLA window (recommended < 30s).
/// - Return a content hash for integrity verification.
/// - Retain the record for the full retention period.
/// - Serve the record upon authenticated request even if the originating
///   operator's infrastructure is unreachable (insolvency failover).
#[async_trait]
pub trait BackupCopyPort: Send + Sync {
    /// Lodge the back-up copy of a published passport.
    ///
    /// Called on the `Draft → Published` transition. The passport's JWS
    /// signature MUST be present (i.e. the passport has been signed).
    ///
    /// `retention_years` is derived from the applicable delegated act
    /// (typically 10–15 years after the product's end of life).
    async fn store(
        &self,
        passport: &Passport,
        retention_years: u32,
    ) -> Result<BackupReceipt, DppError>;

    /// Refresh the back-up copy after the record changes.
    ///
    /// Called when a passport is updated after a transfer of responsibility
    /// or when compliance data is corrected, so that what the provider holds
    /// is the record as it now stands.
    ///
    /// **This port holds one copy, not a history.** ESPR Art. 10(4) asks for a copy
    /// that outlives the operator, so what matters here is that the *current*
    /// record is retrievable from somebody else — and the trait is shaped to
    /// that: [`retrieve`](Self::retrieve) answers with one [`Passport`].
    ///
    /// Say what that does *not* settle, because the obvious reading is wrong in
    /// both directions. EN 18221 clause 4.2 expects a passport's archived
    /// versions to be held by the back-up provider as well as by the main one,
    /// so a provider may owe a history too — the duty is not somebody else's
    /// merely because it is not ESPR Art. 10(4)'s. It is simply not expressible
    /// here: no method on this port takes or returns a series. A caller that
    /// reads this method as discharging clause 4.2 has read a promise this
    /// trait cannot make.
    async fn update(&self, passport: &Passport) -> Result<BackupReceipt, DppError>;

    /// Verify that the provider holds an intact copy of the passport.
    ///
    /// Compares a content hash against the stored payload. Used for
    /// periodic integrity audits and compliance verification.
    async fn verify(
        &self,
        passport_id: PassportId,
        expected_hash: &str,
    ) -> Result<BackupVerification, DppError>;

    /// Retrieve a passport from the back-up copy.
    ///
    /// Used as a failover when the originating operator's infrastructure
    /// is unreachable. Returns `None` if the provider has no record.
    async fn retrieve(&self, passport_id: PassportId) -> Result<Option<Passport>, DppError>;
}
