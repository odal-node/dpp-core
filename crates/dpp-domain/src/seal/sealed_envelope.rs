//! [`SealedEnvelope`] — the produced seal and the provenance it carries.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::format::SealFormat;

/// A completed qualified seal envelope returned by the QTSP.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealedEnvelope {
    /// AdES format of this seal value.
    pub format: SealFormat,
    /// Base64-encoded seal value as returned by the QTSP.
    pub seal_value: String,
    /// Optional reference to the signing certificate chain.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub signing_cert_ref: Option<String>,
    /// Timestamp when the seal was created.
    pub sealed_at: DateTime<Utc>,
    /// True when this envelope was produced by `GhostSeal` and has no legal validity.
    pub placeholder: bool,
}
