//! Borrowing views a caller supplies to the lineage substantiation rule.
//!
//! Primitive views rather than domain types, so core and the Wasm plugins each
//! adapt their own representation — typed structs on one side, JSON fields on
//! the other — without this crate depending on either.

use super::evidence::{DocumentEvidence, TransferEvidence, WasteHandoverEvidence};

/// One item of evidence a caller found for one derivation edge.
///
/// Three kinds, and they do not prove the same thing: see
/// [`SubstantiationBasis`](super::SubstantiationBasis) for what each one backs.
///
/// `#[non_exhaustive]`, so that a kind added later is not a breaking change. The
/// one in view is a record from the EU registry, whose interface for transfers is
/// not published yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Evidence<'a> {
    /// A transfer of responsibility on the predecessor, to this passport's
    /// operator.
    Transfer(TransferEvidence<'a>),
    /// A waste handover on the predecessor, under Reg. (EU) 2023/1542 Art. 77(7)
    /// second subparagraph.
    WasteHandover(WasteHandoverEvidence),
    /// A document from the value chain, supplied by this passport's operator.
    Document(DocumentEvidence<'a>),
}

/// One derivation edge, with every item of evidence the caller found for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DerivationEdge<'a> {
    /// The Art. 77(7) operation this edge claims, in wire form.
    pub operation: &'a str,
    /// The evidence the caller matched to this predecessor. Empty means the
    /// caller looked and found nothing.
    pub evidence: &'a [Evidence<'a>],
}
