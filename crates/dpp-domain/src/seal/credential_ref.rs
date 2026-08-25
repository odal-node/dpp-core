//! [`SealCredentialRef`] — which QTSP-held credential a seal was made with.

use serde::{Deserialize, Serialize};

/// A CSC-style reference to a QTSP-held credential. Never contains key material.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SealCredentialRef {
    /// Identifier of the Qualified Trust Service Provider.
    pub qtsp_id: String,
    /// Credential identifier within the QTSP (CSC `credentialID`).
    pub credential_id: String,
}
