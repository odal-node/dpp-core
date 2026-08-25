//! [`PassportCredentialSubject`] — the claims a passport credential attests.

use serde::{Deserialize, Serialize};

/// Claims about the DPP passport being attested.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PassportCredentialSubject {
    /// `urn:uuid:{passport_id}` — the DPP passport being attested.
    pub id: String,
    /// SHA-256 hex digest of the RFC 8785 canonical payload bytes.
    pub payload_hash: String,
}
