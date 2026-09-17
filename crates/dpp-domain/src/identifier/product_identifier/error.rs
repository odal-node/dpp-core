//! How a unique product identifier fails to parse.

/// Anything that can go wrong reading a [`ProductIdentifier`](super::ProductIdentifier).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ProductIdentifierError {
    /// The identification link is not an absolute `http`/`https` URL.
    ///
    /// EN 18219 scheme 2 is a URL scheme, and a relative or non-web value
    /// cannot be resolved by the reader the identifier exists for.
    #[error("identification link is not an absolute http(s) URL: {0}")]
    NotAWebUrl(String),
    /// The value does not have the `did:<method>:<id>` shape W3C DID v1.0
    /// clause 3.1 requires.
    #[error("not a DID: {0}")]
    NotADid(String),
    /// The DID method is not one EN 18219 scheme 3 names.
    ///
    /// Carried rather than accepted, because a method this build cannot resolve
    /// is an identifier nobody can follow to a passport — see the type's note.
    #[error("DID method '{0}' is not one EN 18219 scheme 3 names (web, ethr, ebsi)")]
    UnsupportedDidMethod(String),
    /// The method-specific identifier is empty.
    #[error("DID has no method-specific identifier: {0}")]
    EmptyDidMethodId(String),
}
