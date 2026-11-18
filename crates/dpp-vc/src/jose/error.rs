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
    /// `iss` is absent, the issuer is identified by a URL, and the `kid` is not an
    /// absolute URL (VC-JOSE-COSE §4.2). `None` when a verified token has no
    /// `kid` at all.
    #[error("kid {} is not an absolute URL, and the token has no iss", .kid.as_deref().unwrap_or("(none)"))]
    KidNotAbsolute {
        /// The `kid` that was given, if there was one.
        kid: Option<String>,
    },
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
    /// The protected header carries `crit`. No critical extension is understood
    /// (RFC 7515 §4.1.11), so none is accepted.
    #[error("the protected header carries crit, and no critical extension is supported")]
    UnsupportedCritical,
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
    /// `typ` is required, and names `vc+jwt` for a credential or `vp+jwt` for a
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
    /// The payload is not a credential or presentation document: not base64url,
    /// not a JSON object, or without a usable `@context`.
    #[error("malformed payload: {reason}")]
    MalformedPayload {
        /// What was wrong with it.
        reason: String,
    },
    /// `exp` or `nbf` is present and is not a JSON number, so it cannot be read as a
    /// time. Refused rather than ignored, because ignoring a malformed `exp` would
    /// read as no expiry.
    #[error("{claim} is not a NumericDate")]
    MalformedTemporalClaim {
        /// `"exp"` or `"nbf"`.
        claim: &'static str,
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
    /// The caller's decision for an envelope was neither to verify it nor to skip
    /// it.
    #[error(
        "no decision for envelope {index} ({media_type}, kid {})",
        .kid.as_deref().unwrap_or("(none)")
    )]
    EnvelopeNotDecided {
        /// The envelope's position.
        index: usize,
        /// The media type its `id` names.
        media_type: String,
        /// The `kid` its protected header names, for a `vc+jwt`.
        kid: Option<String>,
    },
    /// The caller asked for an envelope to be verified, and it is not a `vc+jwt`.
    /// This crate verifies no other format.
    #[error("envelope {index} is {media_type}, and only vc+jwt can be verified here")]
    UnsupportedEnvelopeFormat {
        /// The envelope's position.
        index: usize,
        /// The media type its `id` names.
        media_type: String,
    },
    /// A `vc+jwt` envelope failed one of the checks of a credential.
    #[error("envelope {index} was refused: {source}")]
    CredentialRefused {
        /// The envelope's position.
        index: usize,
        /// The check it failed.
        #[source]
        source: Box<JoseError>,
    },
}
