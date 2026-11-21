//! How signing or verifying a VC-JOSE-COSE credential or presentation fails.

use dpp_crypto::jws::KeyAlgorithm;

/// Why a credential or presentation was not signed, or why a token was refused.
///
/// One variant per refusal, so a caller can tell a presenter what is wrong
/// without matching on prose. None carries key material or a signature.
///
/// Verification reports the first check a token fails, in a fixed order: compact
/// form, protected header, signature, `typ` and `cty`, payload, the claims,
/// and only then the envelopes of a presentation.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum JoseError {
    /// What was handed to a signer is not a JSON object.
    #[error("{what} is not a JSON object")]
    NotAnObject {
        /// What was not an object: `"credential"` or `"presentation"`.
        what: &'static str,
    },
    /// The document has top-level members that are neither a term of the VC Data
    /// Model 2.0 base vocabulary, nor a JWT registered claim name, nor declared by
    /// the caller as an extension term.
    #[error("undeclared top-level members: {}", .names.join(", "))]
    UndeclaredMembers {
        /// Every such member, sorted.
        names: Vec<String>,
    },
    /// The payload has a `vc` or `vp` member. VC-JOSE-COSE §1.1.2.1 forbids both in
    /// the claims set of a credential or presentation, whatever the caller declares.
    #[error("the claim {name} must not be present in a credential or presentation")]
    ReservedClaim {
        /// `"vc"` or `"vp"`.
        name: &'static str,
    },
    /// `iss` is absent, the issuer of a credential or the holder of a presentation is
    /// identified by a URL, and the `kid` is not an absolute URL (VC-JOSE-COSE §4.2).
    /// `None` when a verified token has no `kid` at all.
    #[error("kid {} is not an absolute URL, and the token has no iss", .kid.as_deref().unwrap_or("(none)"))]
    KidNotAbsolute {
        /// The `kid` that was given, if there was one.
        kid: Option<String>,
    },
    /// A disclosure path names a claim that must stay in the clear: `@context`, `type`,
    /// `credentialStatus`, `credentialSchema` or `relatedResource`, which VC-JOSE-COSE §3.2
    /// says "SHOULD be disclosed", or `iss`, `aud`, `exp`, `nbf` or `cnf`, which RFC 9901
    /// clause 9.7 forbids an Issuer to make selectively disclosable. The path is refused
    /// whether it names the claim or a part of it.
    #[error("{name} must not be selectively disclosable")]
    UnconcealableClaim {
        /// The top-level claim the path starts at.
        name: String,
    },
    /// A disclosure path selects nothing in the document: a member it does not have, a
    /// position past the end of an array, a step into a value that holds no members, or a
    /// `null` step over an empty array or over an element that lacks the rest of the path.
    /// Refused, and not ignored, so a claim the issuer meant to conceal is never signed in
    /// the clear. The path is given in its JSON form and names no value.
    #[error("the document holds no claim at {path}")]
    DisclosurePathNotFound {
        /// The path, as `ClaimPath` writes it.
        path: String,
    },
    /// The document has a member named `_sd`, `_sd_alg` or `...`, which SD-JWT reserves
    /// (RFC 9901 clauses 4.1.1 and 4.2.1). A reader would take it for the mechanism and
    /// remove it, so what it verifies would not be the document that was signed.
    #[error("the member name {name} is reserved by SD-JWT")]
    ReservedSdJwtMember {
        /// The reserved name.
        name: &'static str,
    },
    /// A holder key was given to an SD-JWT signer, and the document already carries `cnf`.
    /// The signer writes `cnf` from the key (RFC 7800 §3.1) and does not overwrite one the
    /// caller wrote.
    #[error("the document already carries cnf, so no holder key can be added")]
    ConfirmationClaimPresent,
    /// The key could not sign the document.
    #[error("signing failed: {reason}")]
    Signing {
        /// What the signer reported.
        reason: String,
    },
    /// The token is not three base64url-separated segments.
    #[error("not a compact JWS")]
    NotCompactJws,
    /// The protected header is not base64url, or not a JSON object.
    #[error("the protected header is unreadable")]
    UnreadableHeader,
    /// The SD-JWT is not in the combined format of RFC 9901 clause 4: not
    /// `<JWT>~<Disclosure>~...~`, or a Disclosure that cannot be read, or a last segment
    /// that is not a JWT. The reason is the reader's own words and names no claim value.
    #[error("not a readable SD-JWT: {reason}")]
    MalformedSdJwt {
        /// What the reader reported.
        reason: String,
    },
    /// Key Binding is required, and the token carries no Key Binding JWT (RFC 9901 §7.3
    /// step 2), or is a `vc+jwt` envelope, which cannot carry one. Whether Key Binding is
    /// required is the caller's choice, and the presence of the Key Binding JWT never
    /// changes it.
    #[error("Key Binding is required and the token carries no Key Binding JWT")]
    KeyBindingMissing,
    /// Key Binding is required, and the token does not satisfy it (RFC 9901 §7.3 step 5):
    /// the processed payload names no usable Holder key in `cnf`, or the Key Binding JWT
    /// failed a check. The reason names the check, and no claim value.
    #[error("the Key Binding is refused: {reason}")]
    KeyBindingRefused {
        /// What the check reported.
        reason: String,
    },
    /// The Disclosures do not process against the signed payload (RFC 9901 clause 7.1):
    /// one matches no digest, a digest repeats, a Disclosure would overwrite a claim in the
    /// clear, `_sd_alg` names an unsupported hash, and so on. The reason is the reader's
    /// own words and names no claim value.
    #[error("the Disclosures are refused: {reason}")]
    InvalidDisclosures {
        /// What the reader reported.
        reason: String,
    },
    /// The protected header carries `crit`. No critical extension is understood
    /// (RFC 7515 §4.1.11), so none is accepted.
    #[error("the protected header carries crit, and no critical extension is supported")]
    UnsupportedCritical,
    /// The protected header sets `b64` to `false`. RFC 7797 §7 says a JSON Web Token
    /// must not use that value, and `vc+jwt` and `vp+jwt` are JSON Web Tokens. A
    /// header that also carries `crit` is refused as [`JoseError::UnsupportedCritical`]
    /// first.
    #[error("the protected header sets b64 to false, which a JSON Web Token must not do")]
    UnencodedPayload,
    /// The header's `alg` is not the algorithm of the key the token is checked
    /// against, including `none`.
    #[error(
        "the header's alg is {}, and the key signs under {key_alg:?}",
        .header_alg.as_deref().unwrap_or("absent or not a string")
    )]
    AlgorithmMismatch {
        /// What the header says, when it is a string.
        header_alg: Option<String>,
        /// The algorithm the key signs under.
        key_alg: KeyAlgorithm,
    },
    /// The signature does not verify under the key.
    #[error("the signature does not verify")]
    SignatureDoesNotVerify,
    /// The signature could not be checked at all: it is not base64url, or not the
    /// length its algorithm's signatures have.
    #[error("the signature could not be checked: {reason}")]
    Unverifiable {
        /// What the verifier reported.
        reason: String,
    },
    /// `typ` or `cty` does not type the token as the credential or presentation
    /// that was asked for.
    ///
    /// `typ` is required, and names `vc+jwt` (`vc+sd-jwt` for an SD-JWT) for a credential or `vp+jwt` (`vp+sd-jwt`) for a
    /// presentation, with or without the `application/` prefix (RFC 7515 §4.1.9).
    /// `cty`, when present, names `vc` or `vp`.
    #[error(
        "{member} is {}, not {expected}",
        .found.as_deref().unwrap_or("absent")
    )]
    WrongMediaType {
        /// `"typ"` or `"cty"`.
        member: &'static str,
        /// What the header says, as JSON text; `None` when the member is absent.
        found: Option<String>,
        /// What it must name.
        expected: &'static str,
    },
    /// The payload is not a credential or presentation document: not base64url, or
    /// not a JSON object, or an `iss` that is not a string.
    #[error("malformed payload: {reason}")]
    MalformedPayload {
        /// What was wrong with it.
        reason: String,
    },
    /// The document's `@context` is not the VC Data Model 2.0 context set (§4.3): the
    /// first item must be `https://www.w3.org/ns/credentials/v2`, given alone or first
    /// in an ordered set, and each later item a URL or an object.
    #[error("@context {reason}")]
    InvalidContext {
        /// What is wrong with it.
        reason: String,
    },
    /// The document's `type` does not name what it must: a credential's `type` must
    /// include `VerifiableCredential` (§4.5), and a presentation's must include
    /// `VerifiablePresentation` (§4.13). Both are a string or an array of strings.
    #[error("type {reason}")]
    InvalidType {
        /// What is wrong with it.
        reason: String,
    },
    /// A credential's `issuer` is not a URL, or an object with a URL `id` (VC Data
    /// Model 2.0 §4.7). Refused whether or not the credential carries an `iss`.
    #[error("issuer {reason}")]
    InvalidIssuer {
        /// What is wrong with it.
        reason: String,
    },
    /// A presentation's `holder`, when present, is not a URL or an object with a URL `id`
    /// (VC Data Model 2.0 §4.13).
    #[error("holder {reason}")]
    InvalidHolder {
        /// What is wrong with it.
        reason: String,
    },
    /// A credential's `credentialSubject` is not an object or a non-empty array of
    /// objects, or one of its objects has no claim besides its `id` (VC Data Model 2.0
    /// §4.8).
    #[error("credentialSubject {reason}")]
    InvalidCredentialSubject {
        /// What is wrong with it.
        reason: String,
    },
    /// `exp` or `nbf` is present and is not a JSON number, so it cannot be read as a
    /// time. Refused rather than ignored, because ignoring a malformed `exp` would
    /// read as no expiry. A signer refuses an `iat` that is not a number for the same
    /// reason, since RFC 7519 §4.1.6 makes it a NumericDate; a verifier does not read it.
    #[error("{claim} is not a NumericDate")]
    MalformedTemporalClaim {
        /// `"exp"`, `"nbf"` or, at signing, `"iat"`.
        claim: &'static str,
    },
    /// `aud` is present and is neither a string nor an array of strings (RFC 7519
    /// §4.1.3).
    #[error("aud is neither a string nor an array of strings")]
    MalformedAudience,
    /// The token carries `aud`, and it does not name the audience the caller identified as,
    /// or the caller gave none (RFC 7519 §4.1.3: the JWT "MUST be rejected"). The `aud` of
    /// the token is not repeated here.
    #[error("the token's aud does not name the audience this verifier identifies as")]
    AudienceMismatch {
        /// The audience the caller identified as, or `None` when it gave none.
        audience: Option<String>,
    },
    /// `exp` has been reached (RFC 7519 §4.1.4).
    #[error("the token has expired")]
    Expired,
    /// `nbf` has not been reached (RFC 7519 §4.1.5).
    #[error("the token is not valid yet")]
    NotYetValid,
    /// `iss` is present and is not the credential's `issuer`, or its `id` when the
    /// issuer is an object (VC-JOSE-COSE §4.1.2).
    #[error(
        "iss {iss} does not match the credential's issuer {}",
        .issuer.as_deref().unwrap_or("(none)")
    )]
    IssuerMismatch {
        /// The `iss` claim.
        iss: String,
        /// The credential's issuer, when it has one.
        issuer: Option<String>,
    },
    /// An envelope of a presentation is not an Enveloped Verifiable Credential of the
    /// shape VC-JOSE-COSE §3.1.2 and the Data Model require.
    #[error("envelope {index} is malformed: {reason}")]
    MalformedEnvelope {
        /// The envelope's position in `verifiableCredential`.
        index: usize,
        /// What was wrong with it.
        reason: String,
    },
    /// `verify_credentials` was given no key for an envelope. Its `kid` is the `kid` the
    /// envelope's protected header names, when it is a `vc+jwt` or a `vc+sd-jwt` that names one.
    #[error(
        "no key for envelope {index} (kid {})",
        .kid.as_deref().unwrap_or("(none)")
    )]
    CredentialKeyUnknown {
        /// The envelope's position.
        index: usize,
        /// The `kid` its protected header names, for a `vc+jwt` or a `vc+sd-jwt`.
        kid: Option<String>,
    },
    /// An envelope is neither a `vc+jwt` nor a `vc+sd-jwt`, the only formats this crate
    /// verifies.
    #[error("envelope {index} is {media_type}, and only vc+jwt and vc+sd-jwt can be verified here")]
    UnsupportedEnvelopeFormat {
        /// The envelope's position.
        index: usize,
        /// The media type its `id` names.
        media_type: String,
    },
    /// A `vc+jwt` or `vc+sd-jwt` envelope failed one of the checks of a credential.
    #[error("envelope {index} was refused: {source}")]
    CredentialRefused {
        /// The envelope's position.
        index: usize,
        /// The check it failed.
        #[source]
        source: Box<JoseError>,
    },
}
