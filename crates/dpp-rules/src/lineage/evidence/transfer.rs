//! [`TransferEvidence`] — a transfer of responsibility on the predecessor.

/// What a caller found out about a transfer of responsibility on the
/// predecessor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransferEvidence<'a> {
    /// The transfer's reason, in wire form (`"repurposing"`, …).
    ///
    /// Compared as a string against the edge's operation because the two
    /// vocabularies are defined in a crate this one must not depend on. They are
    /// pinned byte-identical on the core side.
    pub reason: &'a str,
    /// The DID of the operator taking responsibility on.
    pub incoming_operator_did: &'a str,
    /// Whether the **outgoing** operator's authorisation is present.
    ///
    /// This is the predecessor's own signature, and the load-bearing half — see
    /// [`crate::lineage::substantiation`] for why the other half is not
    /// corroboration.
    pub outgoing_authorised: bool,
    /// Whether the handover was accepted and completed, rather than left
    /// pending, rejected or cancelled.
    pub accepted: bool,
}
