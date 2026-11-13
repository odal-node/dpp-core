//! [`TransferChain`] — the append-only history of responsibility transfers.
//!
//! A chain is validated on construction, on append and on read. [`TransferChain::new`]
//! starts an empty chain, which has nothing to check. Appending a record runs the
//! check on the chain with that record added, and a record that fails it is
//! refused. Reading a chain through `Deserialize` runs the check on every record,
//! and a stored chain that fails it is an error, not a chain with its history
//! repaired.
//!
//! The reason is what the chain answers. [`TransferChain::current_operator`] names
//! the responsible economic operator, and a chain that holds a record for another
//! passport, a broken operator sequence, a record after a pending one or a record
//! with more than one outcome gives no single operator to name. A record that is
//! both completed and rejected, for instance, reports itself as rejected while
//! `current_operator` counts it as the hand-over that happened. Nothing in the stored
//! data says which record is right, so the chain does not hold one.
//!
//! The fields are private for the same reason. The history is read through
//! [`TransferChain::records`] and [`TransferChain::pending`]. The pending record is
//! moved on through [`TransferChain::complete_pending`],
//! [`TransferChain::cancel_pending`] and [`TransferChain::reject_pending`], and none
//! of them can leave the chain in a state the check would refuse.

use serde::{Deserialize, Serialize};

use super::error::{ChainFault, TransferError};
use super::record::TransferRecord;
use super::status::TransferStatus;
use crate::operator::ResponsibleOperator;
use crate::passport::PassportId;

/// The complete history of responsibility transfers for a DPP.
///
/// Maintained as an append-only log. Once a transfer is completed,
/// it cannot be modified or removed.
///
/// Every record belongs to this passport, each `from_operator` is the operator the
/// records before it left in place, only the last record may be pending, no record
/// carries more than one outcome, and a completed record carries both signatures.
/// The chain is checked on construction, on append and on read (see the module
/// docs).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", try_from = "TransferChainRaw")]
pub struct TransferChain {
    passport_id: PassportId,
    original_operator: ResponsibleOperator,
    transfers: Vec<TransferRecord>,
}

/// The stored shape of a [`TransferChain`], read before the chain is checked.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferChainRaw {
    passport_id: PassportId,
    original_operator: ResponsibleOperator,
    transfers: Vec<TransferRecord>,
}

impl TryFrom<TransferChainRaw> for TransferChain {
    type Error = TransferError;

    fn try_from(raw: TransferChainRaw) -> Result<Self, Self::Error> {
        Self::validate(
            raw.passport_id,
            &raw.original_operator,
            raw.transfers.iter(),
        )
        .map_err(|fault| TransferError::InvalidChain {
            passport_id: raw.passport_id.to_string(),
            index: fault.index,
            transfer_id: fault.transfer_id.to_string(),
            reason: Box::new(fault.reason),
        })?;
        Ok(Self {
            passport_id: raw.passport_id,
            original_operator: raw.original_operator,
            transfers: raw.transfers,
        })
    }
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

    /// The passport this chain belongs to.
    pub fn passport_id(&self) -> PassportId {
        self.passport_id
    }

    /// The original responsible operator (at first publication).
    pub fn original_operator(&self) -> &ResponsibleOperator {
        &self.original_operator
    }

    /// Every transfer record, oldest first, terminal ones included.
    pub fn records(&self) -> &[TransferRecord] {
        &self.transfers
    }

    /// The pending record, if one is awaiting acceptance or completion.
    ///
    /// A chain holds at most one, because no record may follow a pending one.
    pub fn pending(&self) -> Option<&TransferRecord> {
        self.transfers.iter().find(|record| is_pending(record))
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
    /// The chain is checked with the record added, by the same check a read
    /// applies. The record is refused if it names another passport, if its
    /// `from_operator` is not the current responsible operator, or if another
    /// transfer is still pending. On error the chain is unchanged.
    pub fn initiate_transfer(&mut self, record: TransferRecord) -> Result<(), TransferError> {
        let records = self.transfers.iter().chain(std::iter::once(&record));
        Self::validate(self.passport_id, &self.original_operator, records)
            .map_err(|fault| fault.reason)?;
        self.transfers.push(record);
        Ok(())
    }

    /// The incoming operator rejects the pending transfer.
    ///
    /// Refuses with [`TransferError::NoPendingTransfer`] when nothing is pending,
    /// and with the record's own error when it cannot be rejected. On error the
    /// chain is unchanged.
    pub fn reject_pending(&mut self) -> Result<&TransferRecord, TransferError> {
        self.update_pending(TransferRecord::reject)
    }

    /// The outgoing operator cancels the pending transfer.
    ///
    /// Refuses with [`TransferError::NoPendingTransfer`] when nothing is pending,
    /// and with the record's own error when it cannot be cancelled. On error the
    /// chain is unchanged.
    pub fn cancel_pending(&mut self) -> Result<&TransferRecord, TransferError> {
        self.update_pending(TransferRecord::cancel)
    }

    /// Attest that the incoming operator accepted the pending transfer, and
    /// complete it.
    ///
    /// The attestation is a host's record of the acceptance step, as described on
    /// [`TransferRecord::node_acceptance_attestation`]. A record that already carries
    /// a different attestation is refused, because an attestation is never replaced.
    /// Giving the same one again is accepted. Refuses with
    /// [`TransferError::NoPendingTransfer`] when nothing is pending, and leaves the
    /// chain unchanged on any error.
    pub fn complete_pending(
        &mut self,
        node_acceptance_attestation: String,
    ) -> Result<&TransferRecord, TransferError> {
        self.update_pending(|record| {
            if record
                .node_acceptance_attestation
                .as_deref()
                .is_some_and(|existing| existing != node_acceptance_attestation)
            {
                return Err(TransferError::InvalidState {
                    current: record.status(),
                    action: "replace the acceptance attestation".into(),
                });
            }
            record.node_acceptance_attestation = Some(node_acceptance_attestation);
            record.complete()
        })
    }

    /// Applies `step` to a copy of the pending record, and keeps the copy only if
    /// the step succeeds, so an error leaves the chain as it was.
    fn update_pending(
        &mut self,
        step: impl FnOnce(&mut TransferRecord) -> Result<(), TransferError>,
    ) -> Result<&TransferRecord, TransferError> {
        let index = self.pending_index()?;
        let mut record = self.transfers[index].clone();
        step(&mut record)?;
        self.transfers[index] = record;
        Ok(&self.transfers[index])
    }

    fn pending_index(&self) -> Result<usize, TransferError> {
        self.transfers
            .iter()
            .position(is_pending)
            .ok_or(TransferError::NoPendingTransfer)
    }

    /// Checks every record in order, and returns the first that breaks the chain's
    /// invariant.
    ///
    /// A record must name this passport, and it may carry at most one outcome. A
    /// record with two of `completed_at`, `rejected_at` and `cancelled_at` reports
    /// itself as rejected or cancelled, while `current_operator` counts it as the
    /// hand-over, so nothing can say what happened to it. A record with `completed_at`
    /// set must also carry both signatures, or it reports `Initiated` while holding an
    /// outcome. Its `from_operator` must be the operator that the completed records
    /// before it leave in place. No record may follow a pending one, so at most the
    /// last record is pending. The passport, operator and pending checks are the ones
    /// `initiate_transfer` made record by record. The outcome checks are applied to the
    /// whole history too.
    fn validate<'a>(
        passport_id: PassportId,
        original_operator: &ResponsibleOperator,
        records: impl IntoIterator<Item = &'a TransferRecord>,
    ) -> Result<(), ChainFault> {
        let mut current = original_operator;
        let mut previous_pending = false;
        for (index, record) in records.into_iter().enumerate() {
            let fault = |reason: TransferError| ChainFault {
                index,
                transfer_id: record.transfer_id,
                reason,
            };
            if record.passport_id != passport_id {
                return Err(fault(TransferError::PassportMismatch {
                    expected: passport_id.to_string(),
                    got: record.passport_id.to_string(),
                }));
            }
            let outcomes = [record.completed_at, record.rejected_at, record.cancelled_at];
            if outcomes.into_iter().flatten().count() > 1 {
                return Err(fault(TransferError::InvalidState {
                    current: record.status(),
                    action: "record more than one outcome".into(),
                }));
            }
            if record.completed_at.is_some() && !record.is_complete() {
                return Err(fault(TransferError::InvalidState {
                    current: record.status(),
                    action: "record a completion without both signatures".into(),
                }));
            }
            if record.from_operator.did != current.did {
                return Err(fault(TransferError::OperatorMismatch {
                    expected: current.did.clone(),
                    got: record.from_operator.did.clone(),
                }));
            }
            if previous_pending {
                return Err(fault(TransferError::TransferAlreadyPending));
            }
            previous_pending = is_pending(record);
            if record.is_complete() {
                current = &record.to_operator;
            }
        }
        Ok(())
    }
}

/// Initiated or accepted: the states in which a record still awaits its outcome.
fn is_pending(record: &TransferRecord) -> bool {
    matches!(
        record.status(),
        TransferStatus::Initiated | TransferStatus::Accepted
    )
}
