//! [`ArchiveReceipt`] — what a holder returns once it has archived a version.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::passport::PassportId;

/// Confirmation that a version is archived.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveReceipt {
    /// The passport the archived version belongs to.
    pub passport_id: PassportId,
    /// The instant the archived version was replaced. With the passport, this
    /// identifies the version.
    pub superseded_at: DateTime<Utc>,
    /// SHA-256 of the RFC 8785 canonical form of the archived document, as
    /// lower-case hexadecimal.
    pub content_hash: String,
    /// When the holder accepted the version. Against
    /// [`superseded_at`](Self::superseded_at) this is how far behind a holder that
    /// lags was when it took this one.
    pub archived_at: DateTime<Utc>,
}
