//! [`ArchiveReceipt`] — what an archive returns once it has taken a record.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::passport::PassportId;

/// The archive deadline for a passport retained `years` from `now` — the
/// shared 365-day-per-year approximation used by every archive adapter.
pub(crate) fn retention_deadline(now: DateTime<Utc>, years: u32) -> DateTime<Utc> {
    now + chrono::Duration::days(365 * i64::from(years))
}

/// Confirmation receipt from the third-party archive.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveReceipt {
    /// Archive-assigned identifier for this stored copy.
    pub archive_id: String,
    /// The passport ID of the archived record.
    pub passport_id: PassportId,
    /// Cryptographic hash (SHA-256) of the archived payload for integrity verification.
    pub content_hash: String,
    /// Timestamp when the archive accepted the record.
    pub archived_at: DateTime<Utc>,
    /// The retention period end date (derived from the applicable delegated act).
    /// The archive MUST retain the record until at least this date.
    pub retention_until: DateTime<Utc>,
}
