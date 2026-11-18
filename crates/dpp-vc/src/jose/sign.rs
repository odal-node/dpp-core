//! Signing a credential or a presentation as a VC-JOSE-COSE compact JWS.

use serde_json::{Map, Value};

use dpp_crypto::jws::signer::sign_typed_with;
use dpp_crypto::keystore::KeyEntry;

use super::error::JoseError;
use super::properties::{
    check_context, check_credential_properties, check_presentation_properties,
};
use super::verify::{
    CREDENTIAL_CTY, CREDENTIAL_TYP, PRESENTATION_CTY, PRESENTATION_TYP, check_holder_kid,
    check_issuer, check_temporal_claims,
};
use super::vocabulary::{CREDENTIAL_TERMS, PRESENTATION_TERMS, check_members};

/// Sign a credential as a `vc+jwt` (VC-JOSE-COSE §3.1.1), and return the compact
/// JWS.
///
/// The payload is `credential` as given, in RFC 8785 canonical form, with no claim
/// added: the unsecured credential is the unencoded JWS payload (§3.1.1). The
/// protected header carries the key's `alg`, the `kid` given, `typ: vc+jwt` and
/// `cty: vc`.
///
/// # Refused before signing
///
/// - A top-level member that is not in [`CREDENTIAL_TERMS`], is not a JWT
///   registered claim name (`iss`, `sub`, `aud`, `exp`, `nbf`, `iat`, `jti`, RFC
///   7519 §4.1), and is not named in `extension_terms`. An issuer that means to
///   sign a term of its own declares it: the discipline is the issuer's, and
///   [`verify_credential`](super::verify_credential) ignores unknown members as
///   §3.1.3 requires.
/// - A `vc` or a `vp` member, even when `extension_terms` names it (§1.1.2.1).
/// - An `@context` that is not the VC Data Model 2.0 context set (§4.3), and a `type`,
///   `issuer` or `credentialSubject` that a credential must carry (§4.5, §4.7, §4.8).
/// - An `iss` that is not a string, or that is present and is not the credential's
///   `issuer` (or its `id` when the issuer is an object) (§4.1.2).
/// - A `kid` that is not an absolute URL when the credential has no `iss` and its
///   issuer is a URL (§4.2).
/// - An `exp` or `nbf` that is present and is not a JSON number (RFC 7519 §2).
///
/// Each of these is what [`verify_credential`](super::verify_credential) refuses for the
/// same document, under the same error. Signing compares no time, so an `exp` or `nbf`
/// that is a number is signed whatever `now` a verifier later uses.
///
/// `iat`, `exp` and `nbf` are the caller's to put in the credential, or not: this
/// function stamps nothing, so the same input gives the same token.
///
/// # Errors
///
/// A [`JoseError`] naming what was refused, or [`JoseError::Signing`] when the
/// signer failed.
pub fn sign_credential(
    credential: &Value,
    key: &KeyEntry,
    kid: &str,
    extension_terms: &[&str],
) -> Result<String, JoseError> {
    let document = object(credential, "credential")?;
    check_members(document, &CREDENTIAL_TERMS, extension_terms)?;
    check_context(document)?;
    check_temporal_claims(document)?;
    check_issuer(document, Some(kid))?;
    check_credential_properties(document)?;
    seal(credential, key, kid, CREDENTIAL_TYP, CREDENTIAL_CTY)
}

/// Sign a presentation as a `vp+jwt` (VC-JOSE-COSE §3.1.2), and return the compact
/// JWS.
///
/// The members are checked as [`sign_credential`] checks them, against
/// [`PRESENTATION_TERMS`]. The credentials a presentation holds are given already
/// enveloped, as Enveloped Verifiable Credential objects: they are secured by their
/// own signature, and this function does not open them. [`verify_presentation`](super::verify_presentation)
/// checks each envelope's shape, and [`VerifiedPresentation::verify_credentials`](super::VerifiedPresentation::verify_credentials)
/// verifies each credential under its issuer's key.
///
/// The protected header carries `typ: vp+jwt` and `cty: vp`.
///
/// # Refused before signing
///
/// The same members, `@context`, and `exp`/`nbf` rules as [`sign_credential`], and a
/// `type` that does not include `VerifiablePresentation` (VC DM 2.0 §4.13). A `kid` that
/// is not an absolute URL is refused when the presentation has no `iss` and its `holder`
/// is a URL, by the rule of §4.2 that a credential's issuer is held to.
///
/// # Errors
///
/// A [`JoseError`] naming what was refused, or [`JoseError::Signing`] when the
/// signer failed.
pub fn sign_presentation(
    presentation: &Value,
    key: &KeyEntry,
    kid: &str,
    extension_terms: &[&str],
) -> Result<String, JoseError> {
    let document = object(presentation, "presentation")?;
    check_members(document, &PRESENTATION_TERMS, extension_terms)?;
    check_context(document)?;
    check_temporal_claims(document)?;
    check_holder_kid(document, Some(kid))?;
    check_presentation_properties(document)?;
    seal(presentation, key, kid, PRESENTATION_TYP, PRESENTATION_CTY)
}

fn object<'a>(
    document: &'a Value,
    what: &'static str,
) -> Result<&'a Map<String, Value>, JoseError> {
    document.as_object().ok_or(JoseError::NotAnObject { what })
}

fn seal(
    document: &Value,
    key: &KeyEntry,
    kid: &str,
    typ: &str,
    cty: &str,
) -> Result<String, JoseError> {
    sign_typed_with(key, document, kid, Some(typ), Some(cty)).map_err(|e| JoseError::Signing {
        reason: e.to_string(),
    })
}
