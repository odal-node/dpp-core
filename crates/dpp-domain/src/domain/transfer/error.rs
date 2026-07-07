//! [`TransferError`] — errors from transfer-of-responsibility operations.

use super::status::TransferStatus;

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
