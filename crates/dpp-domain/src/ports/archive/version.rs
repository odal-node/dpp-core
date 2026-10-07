//! [`ArchivedVersion`] — one archived version of a passport.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::passport::PassportId;

/// A passport as it stood until the moment a change replaced it.
///
/// The version was current from the moment the previous one was superseded, or
/// from the passport's creation for the first, until
/// [`superseded_at`](Self::superseded_at), and no longer. The interval is
/// half-open: at `superseded_at` itself the next version is the current one.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchivedVersion {
    /// The passport this is a version of.
    pub passport_id: PassportId,
    /// The passport as it stood, whole and as written. Never a typed
    /// `Passport`: see the module docs for why an archive keeps the document
    /// rather than reading it through a struct.
    pub doc: serde_json::Value,
    /// The instant a change replaced this version.
    pub superseded_at: DateTime<Utc>,
    /// SHA-256 of the RFC 8785 canonical form of [`doc`](Self::doc), as lower-case
    /// hexadecimal. The same definition `BackupReceipt::content_hash` uses.
    pub content_hash: String,
}
