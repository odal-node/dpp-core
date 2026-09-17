//! [`BackupStatus`] — where a record stands with the provider.

use serde::{Deserialize, Serialize};

/// Status of a passport record within the third-party back-up copy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[non_exhaustive]
pub enum BackupStatus {
    /// Record is stored and accessible.
    Active,
    /// Record has been updated (e.g. after a transfer of responsibility).
    Updated,
    /// Record is within the retention-locked period and cannot be removed.
    RetentionLocked,
    /// Retention period has expired; record may be purged by the provider.
    Expired,
}
