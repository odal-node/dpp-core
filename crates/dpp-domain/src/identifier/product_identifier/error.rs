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
    /// The identification link has a character outside ASCII in its host.
    ///
    /// EN 18219:2026 clause 4.3.2 makes a unique identifier an ISO/IEC 646 string.
    /// The remedy is the host's ASCII (punycode) form, and it is the caller's to
    /// apply: the identifier stored has to be the one that is printed, so the
    /// link is refused and not converted.
    #[error(
        "identification link has a non-ASCII character ({character:?}) in its host; write the \
         host in its ASCII (punycode) form: {url}"
    )]
    NonAsciiHost {
        /// The link as it was given.
        url: String,
        /// The first character outside ASCII in the host.
        character: char,
    },
    /// The identification link has a character outside printable ASCII that is
    /// not in its host: a non-ASCII character in the path, the query, the
    /// fragment or the user information, or an ASCII control character.
    ///
    /// The remedy is percent-encoding it, and it is the caller's to apply, for
    /// the reason given on [`Self::NonAsciiHost`].
    #[error(
        "identification link has a character outside printable ASCII ({character:?}); \
         percent-encode it: {url}"
    )]
    NotPrintableAscii {
        /// The link as it was given.
        url: String,
        /// The first character outside printable ASCII.
        character: char,
    },
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
