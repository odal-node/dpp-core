//! [`EolEvent`] — the recorded end-of-life declaration.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::deactivation_reason::DeactivationReason;
use crate::domain::passport::PassportId;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EolEvent {
    /// The passport being declared end-of-life.
    pub passport_id: PassportId,
    /// The typed reason for deactivation.
    pub reason: DeactivationReason,
    /// DID of the operator declaring EOL (provenance).
    pub declared_by: String,
    /// When EOL was declared.
    pub declared_at: DateTime<Utc>,
    /// Optional recovered-material summary for circularity reporting
    /// (recovered-content shares etc.; Battery Annex XIII). Free-form here; a
    /// product group schema constrains it where the act demands.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material_recovery: Option<serde_json::Value>,
    /// Free-text notes (conditions, references).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl EolEvent {
    /// Construct an EOL event stamped `declared_at = now`.
    #[must_use]
    pub fn new(
        passport_id: PassportId,
        reason: DeactivationReason,
        declared_by: impl Into<String>,
    ) -> Self {
        Self {
            passport_id,
            reason,
            declared_by: declared_by.into(),
            declared_at: Utc::now(),
            material_recovery: None,
            notes: None,
        }
    }

    /// True when this EOL is a destruction — which must carry a derogation.
    #[must_use]
    pub fn requires_derogation(&self) -> bool {
        matches!(self.reason, DeactivationReason::Destroyed { .. })
    }
}
