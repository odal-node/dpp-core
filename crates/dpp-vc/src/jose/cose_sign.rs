//! Signing a credential or a presentation as a VC-JOSE-COSE `vc+cose` or `vp+cose`
//! (§3.3.1, §3.3.2).
//!
//! §3.3.1 and §3.3.2 say the issuer "MUST use COSE_Sign1 as specified in [RFC9052]" and that
//! "the unsecured verifiable credential is the unencoded COSE_Sign1 payload". So the payload
//! is the document, as UTF-8 JSON, and not a claims set: no JWT claim is written into it, and
//! no CWT claim is written into the protected header either.

use serde_json::Value;

use dpp_crypto::cose::{CoseError, sign_sign1};
use dpp_crypto::keystore::KeyEntry;

use super::cose_verify::{CREDENTIAL_COSE_TYP, PRESENTATION_COSE_TYP};
use super::error::JoseError;
use super::properties::{
    check_context, check_credential_properties, check_holder, check_presentation_properties,
};
use super::sign::object;
use super::verify::{CREDENTIAL_CTY, PRESENTATION_CTY, check_cose_holder_kid, check_cose_kid};
use super::vocabulary::{CREDENTIAL_TERMS, PRESENTATION_TERMS, check_members_cose};

/// Sign a credential as a `vc+cose` (VC-JOSE-COSE §3.3.1), and return the tagged `COSE_Sign1`.
///
/// The payload is `credential` in RFC 8785 canonical form, as UTF-8 JSON, with nothing added:
/// "The unsecured verifiable credential is the unencoded COSE_Sign1 payload" (§3.3.1). The
/// protected header carries the key's `alg`, the `kid` given as the UTF-8 bytes of the
/// verification method identifier, `typ` (16) `application/vc+cose`, and the content type (3)
/// `application/vc`, which §3.3.1 says SHOULD be those values. No CWT claim is written.
///
/// # Refused before signing
///
/// - A top-level member that is not in [`CREDENTIAL_TERMS`] and is not named in
///   `extension_terms`. The payload is not a JWT Claims Set, so the JWT registered claim names
///   that [`sign_credential`](super::sign_credential) allows are undeclared terms here, and
///   `vc` and `vp` are refused as undeclared terms, not by §1.1.2.1, which is about a JWT
///   Claims Set.
/// - An `@context` that is not the VC Data Model 2.0 context set (§4.3), and a `type`,
///   `issuer` or `credentialSubject` that a credential must carry (§4.5, §4.7, §4.8).
/// - A `kid` that is not an absolute URL when the credential's issuer is a URL (§4.2). The
///   message has no `iss`, so the rule always applies. Verification does not hold a `kid` to
///   it: see [`verify_credential_cose`](super::verify_credential_cose).
///
/// # Errors
///
/// A [`JoseError`] naming what was refused, or [`JoseError::Signing`] when the signer failed,
/// which includes an integer beyond ±(2^53 − 1) anywhere in the document: RFC 8785 writes
/// numbers as IEEE 754 doubles, so it would be signed as another number.
pub fn sign_credential_cose(
    credential: &Value,
    key: &KeyEntry,
    kid: &str,
    extension_terms: &[&str],
) -> Result<Vec<u8>, JoseError> {
    let document = object(credential, "credential")?;
    check_members_cose(document, &CREDENTIAL_TERMS, extension_terms)?;
    check_context(document)?;
    check_credential_properties(document)?;
    check_cose_kid(document, kid)?;
    seal(credential, key, kid, CREDENTIAL_COSE_TYP, CREDENTIAL_CTY)
}

/// Sign a presentation as a `vp+cose` (VC-JOSE-COSE §3.3.2), and return the tagged
/// `COSE_Sign1`.
///
/// The members are checked as [`sign_credential_cose`] checks them, against
/// [`PRESENTATION_TERMS`], with a `type` that includes `VerifiablePresentation` and a `holder`,
/// when present, that is a URL or an object with a URL `id`. The `kid` must be an absolute URL
/// when the holder is a URL. The credentials the presentation holds are given already
/// enveloped and are not opened. The protected header carries `typ` `application/vp+cose` and
/// the content type `application/vp`.
///
/// # Errors
///
/// A [`JoseError`] naming what was refused, or [`JoseError::Signing`] when the signer failed,
/// which includes an integer beyond ±(2^53 − 1) anywhere in the document: RFC 8785 writes
/// numbers as IEEE 754 doubles, so it would be signed as another number.
pub fn sign_presentation_cose(
    presentation: &Value,
    key: &KeyEntry,
    kid: &str,
    extension_terms: &[&str],
) -> Result<Vec<u8>, JoseError> {
    let document = object(presentation, "presentation")?;
    check_members_cose(document, &PRESENTATION_TERMS, extension_terms)?;
    check_context(document)?;
    check_holder(document)?;
    check_cose_holder_kid(document, kid)?;
    check_presentation_properties(document)?;
    seal(
        presentation,
        key,
        kid,
        PRESENTATION_COSE_TYP,
        PRESENTATION_CTY,
    )
}

fn seal(
    document: &Value,
    key: &KeyEntry,
    kid: &str,
    typ: &str,
    cty: &str,
) -> Result<Vec<u8>, JoseError> {
    // The canonicalizer the `vc+jwt` signer uses, which refuses an integer that an RFC 8785
    // double cannot hold exactly rather than sign a different number.
    let payload = dpp_crypto::jws::canonicalize(document).map_err(|e| JoseError::Signing {
        reason: e.to_string(),
    })?;
    sign_sign1(
        key,
        &payload,
        kid.as_bytes(),
        Some(&format!("application/{typ}")),
        Some(&format!("application/{cty}")),
    )
    .map_err(|e: CoseError| JoseError::Signing {
        reason: e.to_string(),
    })
}
