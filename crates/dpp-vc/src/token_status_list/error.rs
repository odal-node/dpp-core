//! How reading a Token Status List fails.

use super::TYP;

/// Anything that can go wrong decoding a list, reading a reference, or checking a
/// Status List Token.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TokenStatusListError {
    /// `bits` is not 1, 2, 4 or 8 (draft-ietf-oauth-status-list-21 section 4.1).
    #[error("bits is {0}, and only 1, 2, 4 and 8 are allowed")]
    UnsupportedBits(u64),
    /// `lst` is not base64url without padding.
    #[error("lst is not unpadded base64url")]
    NotBase64Url,
    /// `lst` decoded, and what it holds is not a zlib stream.
    #[error("lst is not a zlib stream: {0}")]
    NotZlib(String),
    /// The compressor failed writing a list.
    #[error("the list could not be compressed: {0}")]
    Compression(String),
    /// The list decompresses to more bytes than the caller allows.
    ///
    /// DEFLATE expands a small input by about three orders of magnitude, and the
    /// draft sets no limit, so the reader stops at the caller's budget instead of
    /// at the end of the expansion.
    #[error("list is larger than {0} bytes")]
    TooLarge(usize),
    /// A status value does not fit in the list's `bits`.
    #[error("status value {value} does not fit in {bits} bits")]
    ValueTooWide {
        /// The value given.
        value: u8,
        /// The width of one entry.
        bits: u8,
    },
    /// The index is past the last entry of the list.
    ///
    /// Section 8.3 step 6: the Referenced Token MUST be rejected.
    #[error("index {idx} is outside a list of {entries} entries")]
    IndexOutOfBounds {
        /// The index asked for.
        idx: u64,
        /// How many entries the list holds.
        entries: u64,
    },
    /// The entry holds a value the draft has not registered and has not set aside
    /// for applications. Section 7 reserves it for future registration, so it
    /// cannot be read as any status, and above all not as valid.
    #[error("status value {0:#04x} is not registered")]
    UnregisteredStatus(u8),
    /// A `status` claim that is meant as a reference is not one.
    ///
    /// The member that is wrong is named: `status`, `status_list`, `idx` or `uri`.
    #[error("the status reference is malformed at '{0}'")]
    MalformedReference(&'static str),
    /// The Status List Token's signature did not verify against the given key.
    #[error("status list token signature did not verify")]
    BadSignature,
    /// The token's `typ` is absent or is not [`TYP`].
    #[error("typ is '{0}', expected '{TYP}'")]
    WrongTyp(String),
    /// A claim section 5.1 gives a type is absent or is not of that type.
    #[error("claim '{0}' is absent or is not of the type the draft gives it")]
    BadClaim(&'static str),
    /// The token's `exp` has been reached.
    #[error("status list token has expired")]
    Expired,
    /// The token's `nbf` has not been reached.
    ///
    /// RFC 7519 clause 4.1.5: the token MUST NOT be accepted before it.
    #[error("status list token is not valid yet")]
    NotYetValid,
    /// The token's protected header sets `b64` to false (RFC 7797), so its payload is
    /// not the base64url of the claims this crate reads.
    #[error("status list token has an unencoded payload (b64 is false)")]
    UnencodedPayload,
    /// The token is the list of another URI than the one the credential names.
    ///
    /// Section 8.3 step 4a: `sub` MUST be equal to the `uri` of the reference.
    /// Without it a validly signed list for some other credential could answer.
    #[error("the token is for '{token}', and the credential refers to '{reference}'")]
    SubjectMismatch {
        /// The token's `sub`.
        token: String,
        /// The reference's `uri`.
        reference: String,
    },
}
