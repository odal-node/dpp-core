//! [`BackupReceipt`] — what a provider returns once it has taken a copy.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::passport::PassportId;

/// The retention deadline for a passport retained `years` from `now` — the
/// shared 365-day-per-year approximation used by every back-up adapter.
pub(crate) fn retention_deadline(now: DateTime<Utc>, years: u32) -> DateTime<Utc> {
    now + chrono::Duration::days(365 * i64::from(years))
}

/// Confirmation receipt from the third-party provider.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupReceipt {
    /// Provider-assigned identifier for this stored copy.
    pub backup_id: String,
    /// The passport ID of the backed-up record.
    pub passport_id: PassportId,
    /// Cryptographic hash (SHA-256) of the stored payload for integrity verification.
    pub content_hash: String,
    /// Timestamp when the provider accepted the record.
    pub stored_at: DateTime<Utc>,
    /// The retention period end date (derived from the applicable delegated act).
    /// The provider MUST retain the record until at least this date.
    pub retention_until: DateTime<Utc>,
}
