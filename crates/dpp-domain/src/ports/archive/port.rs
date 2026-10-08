//! [`ArchivedVersionPort`] — the contract a holder of archived versions
//! implements.

use async_trait::async_trait;
use chrono::{DateTime, Utc};

use super::receipt::ArchiveReceipt;
use super::version::ArchivedVersion;
use crate::error::DppError;
use crate::passport::PassportId;

/// Port trait for archiving the historical versions of a live passport.
///
/// Implemented by the main store, and by a back-up provider alongside
/// [`BackupCopyPort`](crate::ports::backup::BackupCopyPort). The module docs state
/// the contract; the method docs state what each call owes.
#[async_trait]
pub trait ArchivedVersionPort: Send + Sync {
    /// Archive the version of a passport that a change has just replaced.
    ///
    /// `doc` is the passport as it stood immediately before the change, whole and
    /// as written. `superseded_at` is the instant the change took effect. The port
    /// does not read `doc`'s shape: `passport_id` says whose version it is.
    ///
    /// Versions are kept in order of `superseded_at`, and each must be later than
    /// the one before it, since a version is current from the moment its
    /// predecessor was replaced until its own `superseded_at`, and two changes
    /// cannot share an instant. A call that is not later than the latest archived
    /// version is refused with [`DppError::Validation`], **unless** it repeats a
    /// version already held.
    ///
    /// **A retry is safe.** Archiving a version whose `superseded_at` and content
    /// are the same as one already held returns that version's original receipt
    /// and keeps nothing new, so a caller that lost the answer can ask again,
    /// including after later versions have been archived. The same `superseded_at`
    /// with different content is refused with [`DppError::Validation`], because
    /// two versions cannot both be the one that ended then.
    async fn archive(
        &self,
        passport_id: PassportId,
        doc: &serde_json::Value,
        superseded_at: DateTime<Utc>,
    ) -> Result<ArchiveReceipt, DppError>;

    /// Every archived version of a passport, oldest first.
    ///
    /// Returns an empty list for a passport with no archived version. That is not
    /// an error and it does not say the passport is unknown: this port knows only
    /// what has been archived, and a passport that has never changed has nothing
    /// archived. Whole documents come back, with no disclosure policy applied.
    async fn versions(&self, passport_id: PassportId) -> Result<Vec<ArchivedVersion>, DppError>;

    /// The archived version that was current at `at`, if one is archived.
    ///
    /// That is the archived version with the earliest `superseded_at` **after**
    /// `at`. The interval is half-open, so at a version's own `superseded_at` it
    /// is no longer current and the next one is.
    ///
    /// `None` means no archived version was current then, which makes the live
    /// record the answer. **The port cannot say which it is, or whether the
    /// passport existed at `at` at all.** It does not know when a passport was
    /// created, so for an `at` before that it still returns the first archived
    /// version. A caller checks `at` against the live record.
    async fn version_at(
        &self,
        passport_id: PassportId,
        at: DateTime<Utc>,
    ) -> Result<Option<ArchivedVersion>, DppError>;
}
