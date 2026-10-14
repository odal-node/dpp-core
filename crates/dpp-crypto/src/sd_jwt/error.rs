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
    /// A token that carries a Key Binding JWT was asked to be presented.
    ///
    /// Clause 7.2: an Issuer provides an SD-JWT, not an SD-JWT+KB, and a Holder
    /// that receives one MUST reject it. The Key Binding JWT was made over another
    /// selection of Disclosures, so presenting it again would bind nothing.
    #[error("a token that carries a Key Binding JWT cannot be presented")]
    KeyBindingAttached,
    /// Key Binding is required and the token carries no Key Binding JWT.
    ///
    /// Clause 7.3 step 2. Whether it is required is the Verifier's policy, which
    /// MUST NOT depend on whether one arrived (step 1).
    #[error("Key Binding is required and the token carries no Key Binding JWT")]
    KeyBindingMissing,
    /// The processed payload names no Holder key: no `cnf`, or no `jwk` in it.
    ///
    /// Clause 4.1.2: an Issuer enables Key Binding by naming the Holder's public
    /// key in `cnf`. Without one there is nothing to check a signature against,
    /// and nothing for a Holder to sign with.
    #[error("the token names no Holder key in cnf")]
    NoHolderKey,
    /// The Holder key in `cnf` is not an Ed25519 key, the one algorithm this
    /// crate verifies, or declares an algorithm that is not Ed25519.
    #[error("the Holder key in cnf is not an Ed25519 key")]
    UnsupportedHolderKey,
    /// A Holder was asked to bind a presentation with a key other than the one
    /// `cnf` names. Every Verifier would refuse the result (clause 7.3 step 5.c).
    #[error("the signing key is not the Holder key the token names")]
    HolderKeyMismatch,
    /// The Key Binding JWT failed one of the checks of clause 7.3 step 5, named
    /// here: `alg` or the signature (5.b, 5.c), `typ` (5.d), `iat` (5.e), `nonce`
    /// or `aud` (5.f), `sd_hash` (5.g), or the JWT itself, its `exp` or its `nbf`
    /// (5.h).
    #[error("the Key Binding JWT is not valid: {0}")]
    KeyBindingRejected(&'static str),
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
    /// Clause 7.1 step 5: every disclosure must be used. The count is reported
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
    /// A claim path is empty, or names something a path cannot: a segment that is
    /// not a string, a non-negative integer or `null`.
    #[error("not a claim path: it must be a non-empty array of names, positions and null")]
    InvalidClaimPath,
    /// A claim path selects nothing in the token.
    ///
    /// Refused, and not ignored: a Holder who asked for a claim and was silently
    /// not given it has presented less than it meant to, and has no way to know.
    /// The path is reported in its JSON form. The claim it names is not.
    #[error("the token holds no claim at {0}")]
    ClaimNotInToken(String),
    /// Choosing Disclosures by claim path contradicted itself.
    ///
    /// The Disclosures a path needs are found by walking the signed payload, and
    /// then checked against plain navigation of the fully processed payload and
    /// against the presentation they make. This is a failure of one of those checks:
    /// a defect in this crate, never a property of the token.
    #[error("choosing the Disclosures for {0} disagreed with reading the token a second way")]
    SelectionDisagrees(String),
}
