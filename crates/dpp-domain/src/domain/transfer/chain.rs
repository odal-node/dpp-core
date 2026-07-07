//! [`TransferChain`] — the append-only history of responsibility transfers.

use serde::{Deserialize, Serialize};

use super::error::TransferError;
use super::operator::ResponsibleOperator;
use super::record::TransferRecord;
use super::status::TransferStatus;
use crate::domain::passport::PassportId;

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
