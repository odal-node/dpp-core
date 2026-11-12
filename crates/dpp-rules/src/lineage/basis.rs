//! [`SubstantiationBasis`] — what backed a derivation edge.

/// What backed one derivation edge, strongest first.
///
/// The three are reported apart because they do not prove the same thing, and
/// which of them to accept is the caller's policy. Counting a document the
/// claiming operator supplied the same as the predecessor's own signature would
/// let anyone substantiate a claim to derive from anyone else's product.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum SubstantiationBasis {
    /// A transfer of responsibility the predecessor's operator authorised, to
    /// this passport's operator, for the operation the edge claims. The
    /// predecessor corroborates the claim.
    PredecessorTransfer,
    /// A waste handover the predecessor's operator authorised. It corroborates
    /// that the battery left the predecessor's hands into the waste stream, not
    /// that it reached this passport's operator.
    PredecessorWasteHandover,
    /// A document the claiming operator supplied, pinned by its digest. Nobody
    /// but the claiming operator stands behind it.
    SuppliedDocument,
}
