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
    /// The JSON is not one of the two arrays RFC 9901 defines: the three
    /// elements `[salt, name, value]` of clause 4.2.1 for an object property, or
    /// the two elements `[salt, value]` of clause 4.2.2 for an array element.
    #[error("disclosure is not a [salt, name, value] triple or a [salt, value] pair")]
    NotATripleOrPair,
    /// The claim name is one a disclosure may not carry: `_sd` or `...`, which
    /// clause 4.2.1 forbids, or `_sd_alg`, which clauses 4.1.1 and 4.2.1 and step
    /// 3.f make unusable.
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
    /// The segment after the last `~` is not a compact JWS.
    ///
    /// Clause 4: an SD-JWT ends in `~`, or in a Key Binding JWT. A last
    /// Disclosure that lost its closing `~` is neither, and is refused rather than
    /// taken for a Key Binding JWT, which would drop the claim it carries with no
    /// sign that anything was lost.
    #[error("the segment after the last '~' is not a JWT")]
    MalformedKeyBinding,
    /// The JWT payload is not a JSON object.
    #[error("JWT payload is not a JSON object")]
    PayloadNotAnObject,
    /// `_sd_alg` appears inside a nested object.
    ///
    /// Clause 4.1.1: it "MUST NOT be used in any object nested within the
    /// payload", and clause 7.1 step 3.f removes it from the top level only. A
    /// nested one is a claim the issuer was not allowed to make, so it is refused
    /// and not stripped.
    #[error("_sd_alg appears in a nested object")]
    NestedHashAlg,
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
    /// A digest in an `_sd` array matched a two-element Disclosure, or a digest
    /// standing in for an array element matched a three-element one.
    ///
    /// Clause 7.1 steps 3.c.ii.1 and 3.c.iii.1 both say the SD-JWT MUST be
    /// rejected. The shape of a Disclosure says where it belongs, and a token
    /// that puts one somewhere else is not one the issuer made. The digest is
    /// reported and not the Disclosure, which carries a claim value.
    #[error("digest {0} refers to a disclosure of the other kind")]
    WrongDisclosureKind(String),
    /// The processed payload would hold a value deeper than the depth given, which
    /// is the deepest a value sits in a document `serde_json` parses.
    ///
    /// Each Disclosure is parsed on its own, so Disclosures nested inside one
    /// another can go deeper than that limit, and processing recurses on the
    /// depth. A token that deep is refused rather than read. No document
    /// `serde_json` would parse on its own reaches it.
    #[error("the processed payload holds a value more than {0} levels deep")]
    TooDeep(usize),
}
