//! [`BackupVerification`] — the result of a back-up integrity check.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::status::BackupStatus;

/// Verification result from the back-up integrity check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupVerification {
    /// Whether the stored copy matches the provided content hash.
    pub integrity_ok: bool,
    /// Whether the provider confirms the record is still accessible.
    pub accessible: bool,
    /// Current back-up status.
    pub status: BackupStatus,
    /// Timestamp of the last integrity check performed by the provider.
    pub last_verified_at: DateTime<Utc>,
}
