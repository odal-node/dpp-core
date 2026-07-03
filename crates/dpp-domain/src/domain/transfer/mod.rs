//! Transfer of Responsibility model for EU ESPR DPP.
//!
//! When a product undergoes preparation for reuse, repurposing, or
//! remanufacturing, the new economic operator assumes complete responsibility
//! for providing up-to-date DPP information. The infrastructure must track
//! these transfers with full provenance.
//!
//! This module provides:
//! - `ResponsibleOperator` — identifies the current economic operator
//! - `TransferRecord` — a single ownership transfer event
//! - `TransferChain` — the complete history of responsibility transfers
//! - State machine validation for transfer flows

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::passport::PassportId;

#[cfg(test)]
mod tests;

// ─── Operator identity ───────────────────────────────────────────────────

/// Identifies an economic operator responsible for a DPP.
///
/// Under ESPR, the "responsible economic operator" is whoever places or
/// makes the product available on the EU market. This can be the original
/// manufacturer, an importer, a distributor, or a remanufacturer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponsibleOperator {
    /// The operator's DID (e.g. `did:web:acme.example.com`).
    pub did: String,
    /// Human-readable name of the economic operator.
    pub name: String,
    /// The operator's role in the supply chain.
    pub role: OperatorRole,
    /// EU-assigned economic operator identifier, if available.
    pub eu_operator_id: Option<String>,
    /// ISO 3166-1 alpha-2 country code of the operator's establishment.
    pub country: String,
}

/// The role of an economic operator in the DPP supply chain.
///
/// Determines what DPP fields the operator may introduce or update,
/// as specified by the applicable delegated act.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum OperatorRole {
    /// Original equipment manufacturer.
    Manufacturer,
    /// Imports the product into the EU market.
    Importer,
    /// Makes the product available on the market without altering it.
    Distributor,
    /// An EU-established entity authorised to act on behalf of a
    /// non-EU manufacturer.
    AuthorisedRepresentative,
    /// Performs remanufacturing — restores the product to original
    /// or improved specifications.
    Remanufacturer,
    /// Adapts the product for a different purpose than originally intended.
    Repurposer,
    /// Prepares a used product for resale (testing, cleaning, repair).
    PreparerForReuse,
    /// Professional repairer with authorised DPP update rights.
    Repairer,
    /// Processes end-of-life products for material recovery.
    Recycler,
}

// ─── Transfer record ─────────────────────────────────────────────────────

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

/// Status of a transfer-of-responsibility flow.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum TransferStatus {
    /// Transfer initiated by the outgoing operator, awaiting acceptance.
    Initiated,
    /// Incoming operator has accepted responsibility.
    Accepted,
    /// Transfer rejected by the incoming operator.
    Rejected,
    /// Transfer cancelled by the outgoing operator before acceptance.
    Cancelled,
    /// Transfer completed — DPP now under the new operator's control.
    Completed,
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

// ─── Transfer chain ──────────────────────────────────────────────────────

/// The complete history of responsibility transfers for a DPP.
///
/// Maintained as an append-only log. Once a transfer is completed,
/// it cannot be modified or removed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferChain {
    /// The passport this chain belongs to.
    pub passport_id: PassportId,
    /// The original responsible operator (at first publication).
    pub original_operator: ResponsibleOperator,
    /// Ordered list of transfer events (oldest first).
    pub transfers: Vec<TransferRecord>,
}

impl TransferChain {
    /// Create a new chain with an initial operator and no transfers.
    #[must_use]
    pub fn new(passport_id: PassportId, original_operator: ResponsibleOperator) -> Self {
        Self {
            passport_id,
            original_operator,
            transfers: Vec::new(),
        }
    }

    /// Returns the current responsible operator.
    ///
    /// If no completed transfers exist, returns the original operator.
    /// Otherwise, returns the `to_operator` of the most recent completed transfer.
    pub fn current_operator(&self) -> &ResponsibleOperator {
        self.transfers
            .iter()
            .rev()
            .find(|t| t.is_complete())
            .map(|t| &t.to_operator)
            .unwrap_or(&self.original_operator)
    }

    /// Returns the total number of completed transfers.
    pub fn transfer_count(&self) -> usize {
        self.transfers.iter().filter(|t| t.is_complete()).count()
    }

    /// Append a new transfer record to the chain.
    ///
    /// Validates that:
    /// - The `from_operator` matches the current responsible operator.
    /// - No other transfer is currently pending (initiated but not completed/cancelled).
    pub fn initiate_transfer(&mut self, record: TransferRecord) -> Result<(), TransferError> {
        // Check that from_operator matches current
        let current = self.current_operator();
        if record.from_operator.did != current.did {
            return Err(TransferError::OperatorMismatch {
                expected: current.did.clone(),
                got: record.from_operator.did.clone(),
            });
        }

        // Check no pending transfer exists
        let has_pending = self.transfers.iter().any(|t| {
            t.status() == TransferStatus::Initiated || t.status() == TransferStatus::Accepted
        });
        if has_pending {
            return Err(TransferError::TransferAlreadyPending);
        }

        self.transfers.push(record);
        Ok(())
    }
}

// ─── Errors ──────────────────────────────────────────────────────────────

/// Errors specific to transfer-of-responsibility operations.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TransferError {
    /// The `from_operator` on the transfer record doesn't match the
    /// current responsible operator on the chain.
    OperatorMismatch { expected: String, got: String },
    /// A transfer is already pending for this passport.
    TransferAlreadyPending,
    /// The transfer record is not in a state that allows this operation.
    InvalidState {
        current: TransferStatus,
        action: String,
    },
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferError::OperatorMismatch { expected, got } => {
                write!(f, "operator mismatch: expected {expected}, got {got}")
            }
            TransferError::TransferAlreadyPending => {
                write!(f, "a transfer is already pending for this passport")
            }
            TransferError::InvalidState { current, action } => {
                write!(f, "cannot {action}: transfer is in {current:?} state")
            }
        }
    }
}

impl std::error::Error for TransferError {}
