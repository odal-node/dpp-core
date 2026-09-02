//! Borrowing views a caller supplies to the lineage consent rule.
//!
//! Primitive views rather than domain types, so core and the Wasm plugins each
//! adapt their own representation — typed structs on one side, JSON fields on
//! the other — without this crate depending on either.

/// What a caller found out about the transfer it correlated to one edge.
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
    /// [`super::consent`] for why the other half is not evidence of consent.
    pub outgoing_authorised: bool,
    /// Whether the handover was accepted and completed, rather than left
    /// pending, rejected or cancelled.
    pub accepted: bool,
}

/// One derivation edge, with whatever transfer the caller correlated to it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivationEdge<'a> {
    /// The Art. 77(7) operation this edge claims, in wire form.
    pub operation: &'a str,
    /// The transfer the caller matched to this predecessor. `None` means the
    /// caller looked and found nothing.
    pub transfer: Option<TransferEvidence<'a>>,
}
