//! How reading an SD-JWT fails.
//!
//! Both enums live here rather than beside the types they belong to because the
//! two are a single contract: a caller handed a credential from outside gets one
//! or the other, and should be able to see every way that can go without reading
//! the mechanism.

/// Anything that can go wrong reading a disclosure someone else produced.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DisclosureError {
    /// The string is not base64url.
    #[error("disclosure is not base64url")]
    NotBase64,
    /// The decoded bytes are not JSON.
    #[error("disclosure does not decode to JSON")]
    NotJson,
    /// The JSON is not the three-element array clause 4.2.1 requires.
    ///
    /// The two-element form (array-element disclosures, clause 4.2.2) is
    /// deliberately not accepted — see the module documentation.
    #[error("disclosure is not a [salt, name, value] triple")]
    NotATriple,
    /// The claim name is one clause 4.2.1 forbids: `_sd` or `...`.
    #[error("disclosure names a reserved claim")]
    ReservedClaimName,
}

/// Anything that can go wrong reading an SD-JWT.
#[derive(Debug, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum SdJwtError {
    /// Fewer than two `~`-separated segments, so not an SD-JWT at all.
    #[error("not an SD-JWT: fewer than two segments")]
    NotAnSdJwt,
    /// A disclosure segment could not be read.
    #[error(transparent)]
    Disclosure(#[from] DisclosureError),
    /// The issuer-signed JWT is not three dot-separated parts.
    #[error("issuer-signed JWT is malformed")]
    MalformedJwt,
    /// The JWT payload is not a JSON object.
    #[error("JWT payload is not a JSON object")]
    PayloadNotAnObject,
    /// `_sd_alg` names a hash function this does not implement.
    ///
    /// Carried rather than defaulted: clause 4.1.1 permits the claim to be
    /// absent and to mean `sha-256`, but a *present* value naming something else
    /// must not be read as if it said `sha-256`.
    #[error("unsupported _sd_alg: {0}")]
    UnsupportedHashAlg(String),
    /// A disclosure was supplied whose digest appears nowhere in the token.
    ///
    /// Clause 7.1 step 4: every disclosure must be used. The count is reported
    /// rather than the disclosure itself, which carries a claim value.
    #[error("{0} disclosure(s) matched nothing in the token")]
    UnusedDisclosures(usize),
    /// The same digest appeared more than once.
    ///
    /// RFC 9901 clause 4.1: *"The same digest value MUST NOT appear more than
    /// once in the SD-JWT."* Repetition is how a token smuggles a second
    /// meaning past a reader that de-duplicates, so it is refused rather than
    /// collapsed.
    #[error("digest {0} appears more than once")]
    DuplicateDigest(String),
    /// A disclosure would overwrite a claim already present in cleartext.
    ///
    /// Clause 7.1 makes this a rejection condition: the token would otherwise
    /// have two values for one claim and the reader would pick one.
    #[error("disclosure for '{0}' collides with a cleartext claim")]
    ClaimCollision(String),
}
