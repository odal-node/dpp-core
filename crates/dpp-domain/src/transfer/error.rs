//! [`TransferError`] — errors from transfer-of-responsibility operations.

use uuid::Uuid;

use super::status::TransferStatus;

/// Errors specific to transfer-of-responsibility operations.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum TransferError {
    /// The transfer record names a different passport from the chain it is
    /// appended to.
    PassportMismatch { expected: String, got: String },
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
    /// A step was asked of the pending transfer, and the chain has none pending.
    NoPendingTransfer,
    /// A chain read from storage breaks the chain's invariant.
    ///
    /// `index` and `transfer_id` locate the record that breaks it, and `reason`
    /// is the error the same check raises when that record is appended.
    InvalidChain {
        passport_id: String,
        index: usize,
        transfer_id: String,
        reason: Box<TransferError>,
    },
}

impl std::fmt::Display for TransferError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransferError::PassportMismatch { expected, got } => {
                write!(
                    f,
                    "passport mismatch: chain is for {expected}, record is for {got}"
                )
            }
            TransferError::OperatorMismatch { expected, got } => {
                write!(f, "operator mismatch: expected {expected}, got {got}")
            }
            TransferError::TransferAlreadyPending => {
                write!(f, "a transfer is already pending for this passport")
            }
            TransferError::InvalidState { current, action } => {
                write!(f, "cannot {action}: transfer is in {current:?} state")
            }
            TransferError::NoPendingTransfer => {
                write!(f, "no transfer is pending for this passport")
            }
            TransferError::InvalidChain {
                passport_id,
                index,
                transfer_id,
                reason,
            } => {
                write!(
                    f,
                    "transfer chain for passport {passport_id} is invalid at record {index} ({transfer_id}): {reason}"
                )
            }
        }
    }
}

impl std::error::Error for TransferError {}

/// Why [`TransferChain::from_value`](super::TransferChain::from_value) did not
/// return a chain.
///
/// The two cases are different faults and a host answers them differently: a
/// document that is not a transfer chain at all, and a transfer chain whose
/// history breaks the chain's invariant, so that it names no single responsible
/// operator.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ChainReadError {
    /// The value is not the shape of a transfer chain: a member is missing, or
    /// one has the wrong type. The invariant was never checked.
    #[error("not a transfer chain: {0}")]
    Malformed(#[source] serde_json::Error),
    /// The value reads as a transfer chain, and the chain breaks its invariant.
    ///
    /// Always [`TransferError::InvalidChain`], which names the passport and
    /// locates the record that breaks the invariant.
    #[error("{0}")]
    Refused(#[source] TransferError),
}

/// Where a chain's check stopped: the first record that breaks the invariant.
///
/// Crate-private. Appending a record returns its `reason`, and reading a chain
/// wraps it in [`TransferError::InvalidChain`] together with the location.
pub(super) struct ChainFault {
    pub(super) index: usize,
    pub(super) transfer_id: Uuid,
    pub(super) reason: TransferError,
}
