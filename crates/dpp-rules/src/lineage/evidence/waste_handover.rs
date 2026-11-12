//! [`WasteHandoverEvidence`] — a waste handover on the predecessor.

/// What a caller found out about a waste handover on the predecessor: the
/// battery became waste, and responsibility left its operator for a producer, a
/// producer responsibility organisation or a waste management operator, under
/// Reg. (EU) 2023/1542 Art. 77(7) second subparagraph.
///
/// It names no incoming operator to compare, deliberately. The recipient is a
/// waste-stream party, not this passport's operator, so the handover shows that
/// the battery left the predecessor's hands and not that it reached this one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WasteHandoverEvidence {
    /// Whether the outgoing operator's authorisation is present: the
    /// predecessor's own signature, as for a
    /// [`TransferEvidence`](super::TransferEvidence).
    pub outgoing_authorised: bool,
    /// Whether the handover was accepted and completed.
    pub accepted: bool,
}
