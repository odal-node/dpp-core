//! [`TransferRecord`] — a single transfer-of-responsibility event.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::error::TransferError;
use super::operator::ResponsibleOperator;
use super::status::TransferStatus;
use crate::domain::passport::PassportId;

/// The reason for a transfer of DPP responsibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum TransferReason {
    /// Product sold to a new economic operator for market placement.
    Sale,
    /// Product returned to the supply chain (e.g. customer return).
    Return,
    /// Product sent for remanufacturing.
    Remanufacturing,
    /// Product adapted for a different purpose.
    Repurposing,
    /// Product prepared for resale as second-hand.
    PreparationForReuse,
    /// Product imported into the EU by a new importer.
    Import,
    /// Original operator became insolvent; responsibilities assumed by successor.
    InsolvencySuccession,
}

/// A single transfer-of-responsibility event in the DPP lifecycle.
///
/// Each transfer is cryptographically signed by both the outgoing and
/// incoming operators to create an auditable provenance chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferRecord {
    /// Unique identifier for this transfer event.
    pub transfer_id: Uuid,
    /// The passport being transferred.
    pub passport_id: PassportId,
    /// The outgoing (previous) responsible operator.
    pub from_operator: ResponsibleOperator,
    /// The incoming (new) responsible operator.
    pub to_operator: ResponsibleOperator,
    /// The reason for this transfer.
    pub reason: TransferReason,
    /// Compact JWS signature from the outgoing operator, signing over the
    /// transfer payload to authorise the handover.
    pub from_signature: Option<String>,
    /// Compact JWS signature from the incoming operator, accepting
    /// responsibility for the DPP.
    pub to_signature: Option<String>,
    /// Timestamp when the transfer was initiated.
    pub initiated_at: DateTime<Utc>,
    /// Timestamp when the transfer was completed (both parties signed).
    /// `None` if the transfer is still pending acceptance.
    pub completed_at: Option<DateTime<Utc>>,
    /// Timestamp when the incoming operator explicitly rejected the transfer.
    /// Set by [`TransferRecord::reject`]; makes the record terminal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejected_at: Option<DateTime<Utc>>,
    /// Timestamp when the outgoing operator cancelled the transfer.
    /// Set by [`TransferRecord::cancel`]; makes the record terminal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<DateTime<Utc>>,
    /// Free-text notes (e.g. conditions, regulatory references).
    pub notes: Option<String>,
}

impl TransferRecord {
    /// The canonical content both operators sign over: the immutable core
    /// of the transfer, excluding the signatures themselves and the lifecycle
    /// timestamps set *after* signing (`completed_at`/`rejected_at`/`cancelled_at`).
    ///
    /// Both `from_operator` and `to_operator` sign a JWS over the JCS
    /// canonicalisation of this value, so the two signatures bind the same
    /// immutable handover terms. Tampering any bound field invalidates both.
    #[must_use]
    pub fn signing_payload(&self) -> serde_json::Value {
        serde_json::json!({
            "transferId": self.transfer_id,
            "passportId": self.passport_id,
            "fromOperator": self.from_operator,
            "toOperator": self.to_operator,
            "reason": self.reason,
            "initiatedAt": self.initiated_at,
        })
    }

    /// Determine the current status of this transfer.
    ///
    /// Terminal states (`Rejected`, `Cancelled`) take priority over signatures,
    /// so a cancelled transfer that already had the from_signature still reports
    /// `Cancelled` rather than `Initiated`.
    pub fn status(&self) -> TransferStatus {
        if self.rejected_at.is_some() {
            return TransferStatus::Rejected;
        }
        if self.cancelled_at.is_some() {
            return TransferStatus::Cancelled;
        }
        match (&self.from_signature, &self.to_signature, &self.completed_at) {
            (Some(_), Some(_), Some(_)) => TransferStatus::Completed,
            (Some(_), Some(_), None) => TransferStatus::Accepted,
            _ => TransferStatus::Initiated,
        }
    }

    /// Returns `true` if both parties have signed and the transfer is finalised.
    pub fn is_complete(&self) -> bool {
        self.from_signature.is_some() && self.to_signature.is_some() && self.completed_at.is_some()
    }

    /// The incoming operator explicitly rejects the transfer.
    ///
    /// Only valid from `Initiated` state. After rejection the record is terminal;
    /// a new transfer may be initiated on the chain.
    pub fn reject(&mut self) -> Result<(), TransferError> {
        let s = self.status();
        if s != TransferStatus::Initiated {
            return Err(TransferError::InvalidState {
                current: s,
                action: "reject".into(),
            });
        }
        self.rejected_at = Some(Utc::now());
        Ok(())
    }

    /// The outgoing operator cancels the transfer before it completes.
    ///
    /// Valid from `Initiated` or `Accepted` state. After cancellation the
    /// record is terminal; a new transfer may be initiated on the chain.
    pub fn cancel(&mut self) -> Result<(), TransferError> {
        match self.status() {
            TransferStatus::Initiated | TransferStatus::Accepted => {
                self.cancelled_at = Some(Utc::now());
                Ok(())
            }
            s => Err(TransferError::InvalidState {
                current: s,
                action: "cancel".into(),
            }),
        }
    }

    /// Mark the transfer as completed once both parties have signed.
    ///
    /// Only valid from `Accepted` state (both signatures present, no
    /// `completed_at` yet). This is the final step before the incoming
    /// operator becomes the current responsible operator in the chain.
    pub fn complete(&mut self) -> Result<(), TransferError> {
        let s = self.status();
        if s != TransferStatus::Accepted {
            return Err(TransferError::InvalidState {
                current: s,
                action: "complete".into(),
            });
        }
        self.completed_at = Some(Utc::now());
        Ok(())
    }
}
