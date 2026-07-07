//! [`TransferStatus`] — the state of a transfer-of-responsibility flow.

use serde::{Deserialize, Serialize};

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
