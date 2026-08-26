//! [`TransferNotification`] — sent to the EU registry on a transfer of responsibility.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::RegistryValidationError;
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
    /// JWS signature from the outgoing operator, authorising the handover.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub from_signature: Option<String>,
    /// The notifying node's attestation that the incoming operator accepted —
    /// **not** a signature by the incoming operator, who holds no key on that
    /// node. Sending it under its former name, `toSignature`, told the registry
    /// that both parties had authorised the handover, which the notifier is in
    /// no position to state.
    ///
    /// The registry's own record is the authority here: Implementing Regulation
    /// (EU) 2026/1778 Art. 6a transfers a registration between actors whose
    /// identity is verified under its Arts. 4-5, and this notification is a
    /// report to that record rather than evidence for it.
    #[serde(alias = "toSignature", skip_serializing_if = "Option::is_none")]
    pub node_acceptance_attestation: Option<String>,
}

impl TransferNotification {
    /// Validate both operator identifiers and the reason.
    ///
    /// A transfer notification names the two legal persons on either side of a
    /// change of responsibility, so both must satisfy the same rules a
    /// registration's operator does — a legal name, a valid country, and a
    /// structurally sound identifier for the scheme.
    ///
    /// Neither proof field is required here: a transfer is initiated by the
    /// outgoing operator and the acceptance step only runs when the incoming
    /// one accepts, so a notification can legitimately be built before
    /// `node_acceptance_attestation` exists. Whether an unaccepted transfer
    /// should be notified at all is a caller's decision, not a structural one.
    pub fn validate(&self) -> Result<(), RegistryValidationError> {
        self.from_operator.validate()?;
        self.to_operator.validate()?;
        if self.reason.trim().is_empty() {
            return Err(RegistryValidationError::MissingRequiredField(
                "reason".into(),
            ));
        }
        Ok(())
    }
}
