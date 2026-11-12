//! What the lineage substantiation rule reports about each derivation edge.

use super::basis::SubstantiationBasis;

/// Why one item of evidence does not back a derivation edge, or why an edge has
/// none.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubstantiationDefect<'a> {
    /// No evidence of any kind corresponds to this predecessor, so the claim
    /// rests on this operator's own assertion.
    NoEvidence,
    /// The transfer or waste handover carries no authorisation from the outgoing
    /// operator.
    ///
    /// The defect that matters most: that authorisation is the one signature in
    /// the record that is a counterparty's own, so without it the record
    /// corroborates nothing the claiming operator could not have written itself.
    OutgoingAuthorisationMissing,
    /// The handover was never completed — still pending, or rejected, or
    /// cancelled. An abandoned handover does not support a live claim.
    NotAccepted,
    /// The transfer hands responsibility to someone other than this passport's
    /// operator, so it is not evidence that *this* operator took it on.
    IncomingOperatorMismatch {
        /// This passport's operator.
        expected: &'a str,
        /// Who the transfer actually names.
        found: &'a str,
    },
    /// A transfer exists but records a different operation than the edge claims.
    ///
    /// Art. 77(7)'s four operations carry different legal consequences, so the
    /// two must name the same one.
    OperationMismatch {
        /// What the edge claims happened.
        edge_operation: &'a str,
        /// What the transfer records happened.
        transfer_reason: &'a str,
    },
    /// A waste handover is offered for an edge whose operation cannot start from
    /// a waste battery.
    ///
    /// Reg. (EU) 2023/1542 Art. 3(31) defines repurposing over "a battery, that is
    /// not a waste battery". The operation that prepares a waste battery for a
    /// different purpose is preparation for repurposing, Art. 3(30), so a
    /// predecessor that became waste contradicts a `repurposing` edge rather than
    /// backing it.
    WasteHandoverContradictsOperation {
        /// What the edge claims happened.
        edge_operation: &'a str,
    },
    /// A document names no type, so nothing says what it is evidence of.
    DocumentTypeMissing,
    /// A document's digest is not a SHA-256: 64 hexadecimal characters.
    DocumentDigestMalformed,
}

/// The outcome for one derivation edge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeSubstantiation<'a> {
    /// Index into the edges as the caller passed them.
    pub edge_index: usize,
    /// What backed the edge, or the most fundamental reason nothing did.
    pub outcome: Result<SubstantiationBasis, SubstantiationDefect<'a>>,
    /// Index into the edge's evidence of the item the outcome rests on: the one
    /// that backed it, or the one whose defect is reported. `None` only for
    /// [`SubstantiationDefect::NoEvidence`].
    pub evidence_index: Option<usize>,
}
