//! [`DocumentEvidence`] — a document from the value chain.

/// A document from the value chain: what it is, and the digest that pins it.
///
/// The rule cannot read a document, so it cannot tell whether one is genuine or
/// says what its type claims. It checks only that a type is named and that the
/// digest is a SHA-256; that the digest matches a document the operator holds is
/// the caller's to check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DocumentEvidence<'a> {
    /// What the document is, as an identifier the caller chooses (an IRI, for
    /// instance).
    ///
    /// Open on purpose. Art. 77(7) names no evidence, and Implementing
    /// Regulation (EU) 2026/1778 Art. 12(2)(a) has the Commission's semantic
    /// repository specify "links between digital product passport attributes and
    /// underlying evidence communicated through the product value chain". A list
    /// of document types fixed here would anticipate that specification rather
    /// than consume it.
    pub document_type: &'a str,
    /// The hex SHA-256 of the document's bytes.
    pub sha256: &'a str,
}
