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
    /// SHA-256 of the RFC 8785 (JCS) canonical form of the stored passport, as
    /// lower-case hexadecimal, for integrity verification. The archive port
    /// defines its hash the same way, so a version held in either carries one
    /// hash: see [`ArchivedVersionPort`](crate::ports::archive::ArchivedVersionPort).
    pub content_hash: String,
    /// Timestamp when the provider accepted the record.
    pub stored_at: DateTime<Utc>,
    /// The retention period end date (derived from the applicable delegated act).
    /// The provider MUST retain the record until at least this date.
    pub retention_until: DateTime<Utc>,
}
