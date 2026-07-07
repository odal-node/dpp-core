//! [`TransferNotification`] — sent to the EU registry on a transfer of responsibility.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::identifiers::OperatorIdentifier;

/// Notification sent to the EU registry when a transfer of responsibility occurs.
///
/// 🟠 COMPLIANCE-PIN PENDING: checked against the verbatim OJ text (Regulation (EU)
/// 2024/1781) — there is **no distinct "transfer of responsibility" provision** by
/// that name in Articles 9-15. The closest support is the general data-accuracy
/// duty ("the data in the digital product passport shall be accurate, complete and
/// up to date", **Art. 9(1)**) plus the registry-upload duty (**Art. 13(4)**); a
/// dedicated transfer-notice obligation is not textually confirmed. The prior
/// single-article "Article 9" citation is corrected to this honest, narrower basis
/// — this notification is a sound compliance-hygiene design, not a verbatim-cited
/// requirement.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TransferNotification {
    /// The passport being transferred.
    pub passport_id: Uuid,
    /// The registry's ID for this DPP.
    pub registry_id: String,
    /// The operator transferring responsibility.
    pub from_operator: OperatorIdentifier,
    /// The operator receiving responsibility.
    pub to_operator: OperatorIdentifier,
    /// Reason for the transfer (maps to `TransferReason` in dpp-domain).
    pub reason: String,
    /// ISO 8601 timestamp of the transfer.
    pub transferred_at: DateTime<Utc>,
    /// JWS signature from the outgoing operator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_signature: Option<String>,
    /// JWS signature from the incoming operator.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to_signature: Option<String>,
}
