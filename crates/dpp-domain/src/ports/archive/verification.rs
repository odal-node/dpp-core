//! [`ArchiveVerification`] — the result of an archive integrity check.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::status::ArchiveStatus;

/// Verification result from the archive integrity check.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveVerification {
    /// Whether the archived copy matches the provided content hash.
    pub integrity_ok: bool,
    /// Whether the archive confirms the record is still accessible.
    pub accessible: bool,
    /// Current archive status.
    pub status: ArchiveStatus,
    /// Timestamp of the last integrity check performed by the archive.
    pub last_verified_at: DateTime<Utc>,
}
