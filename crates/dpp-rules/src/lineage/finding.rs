//! What the lineage substantiation rule reports about an unsupported edge.

/// Why one derivation edge is not supported by a transfer of responsibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubstantiationDefect<'a> {
    /// No transfer record corresponds to this predecessor at all, so the claim
    /// to have taken responsibility rests on this operator's own assertion.
    NoTransfer,
    /// The record carries no authorisation from the outgoing operator.
    ///
    /// The defect that matters most: that authorisation is the one signature in
    /// the record that is a counterparty's own, so without it the record
    /// corroborates nothing the claiming operator could not have written itself.
    OutgoingAuthorisationMissing,
    /// The handover was never completed — still pending, or rejected, or
    /// cancelled. An abandoned transfer does not support a live claim.
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
}

/// One unsupported edge, identified by its position in the passport's list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SubstantiationFinding<'a> {
    /// Index into the `derived_from` list as the caller passed it.
    pub edge_index: usize,
    /// What is wrong with it.
    pub defect: SubstantiationDefect<'a>,
}
