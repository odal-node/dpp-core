//! Verifying a credential or a presentation secured with COSE, as a `vc+cose` or a `vp+cose`
//! (VC-JOSE-COSE §3.3, §5.3).
//!
//! The payload is the document, and not a claims set: no JWT claim is read out of it. What a
//! JWT carries as `iss`, `aud`, `exp` and `nbf` a COSE message carries, when it carries them,
//! in the CWT Claims header parameter of its protected header (§3.3.3, RFC 9597, RFC 8392).
//! The [`dpp_crypto::cose`] layer verifies the message and returns those claims, and this
//! module applies them as the JOSE form applies the same JWT claims.

use serde_json::{Map, Value};

use dpp_crypto::cose::{CoseError, CwtClaims, VerifiedSign1, verify_sign1};
use dpp_crypto::jws::PublicKey;

use super::error::JoseError;
use super::options::VerifyOptions;
use super::presentation::finish_presentation;
use super::properties::{check_context, check_credential_properties};
use super::verified_presentation::VerifiedPresentation;
use super::verify::{
    CREDENTIAL_CTY, PRESENTATION_CTY, check_cwt_audience, check_instant, check_issuer_claim,
};
use crate::common::media_type::{self, TypeRefusal};

/// The `typ` of a credential secured with COSE, and of a presentation (VC-JOSE-COSE §3.3.1 and
/// §3.3.2), as the specification writes them: `application/vc+cose` and
/// `application/vp+cose`. A reader accepts the value without the `application/` prefix, as
/// the JOSE form does.
pub(super) const CREDENTIAL_COSE_TYP: &str = "vc+cose";
pub(super) const PRESENTATION_COSE_TYP: &str = "vp+cose";

/// Verify a credential secured as a `vc+cose` (VC-JOSE-COSE §3.3.1), and return the credential
/// it carries.
///
/// `bytes` is the `COSE_Sign1` message, tagged or not. `key` is the issuer's verification key,
/// which the caller resolved by whatever means it trusts: §5 says key discovery "is assumed
/// to have succeeded prior to beginning the verification process", and the `kid` is a hint.
///
/// Checks, in order, refusing at the first that fails:
///
/// 1. the message verifies under `key` as RFC 9052 §4.4 says, with the header rules of
///    [`verify_sign1`] ([`JoseError::Cose`]);
/// 2. `typ` is present and names `vc+cose`, and the content type, if present, names `vc`, each
///    with or without the `application/` prefix and without regard to case. This is the policy
///    the `vc+jwt` form applies to its `typ` and `cty`, though §3.3.1 says SHOULD for both
///    ([`JoseError::WrongMediaType`]);
/// 3. the payload is a JSON object whose `@context` is the VC Data Model 2.0 context set
///    (VC DM 2.0 §4.3). `vc` and `vp` are not refused: §1.1.2.1 is stated for a JWT Claims Set,
///    which this payload is not;
/// 4. under a `now`, the CWT claims `exp` and `nbf` of the protected header, as below;
/// 5. the CWT claim `aud`, as below;
/// 6. the CWT claim `iss`, if present, equals the credential's `issuer`, or its `id` when the
///    issuer is an object (§4.1.2);
/// 7. the credential's `type`, `issuer` and `credentialSubject` (VC DM 2.0 §4.5, §4.7, §4.8).
///
/// Members this function does not understand are ignored (§3.3.3, §5.4), and so are CWT claims
/// other than the six it reads.
///
/// # CWT claims
///
/// RFC 9597 §2 recommends the CWT Claims header parameter (15) "only be used in a protected
/// header", and §1 says a recipient "MUST NOT use the information in the CWT Claims header
/// parameter beyond the integrity protection or authentication afforded", so one in the
/// unprotected header is refused by the COSE layer. The claims are applied as the same JWT
/// claims are in [`verify_credential`](super::verify_credential): `exp` and `nbf` use the
/// leeway in `options`, and neither is read without a `now`; an `aud` is accepted only when it
/// names the audience the caller identifies as (RFC 7519 §4.1.3,
/// which RFC 8392 §3.1.3 adopts), and a message that carries one is refused when the caller
/// gave none. `iat` is never checked.
///
/// # The `kid`
///
/// It is not held to being an absolute URL when there is no `iss`. §4.2 opens "When using
/// controlled identifier documents with this specification", and this function is given the
/// key. The signers do hold their own `kid` to it.
///
/// # Errors
///
/// A [`JoseError`] naming the first check the message failed.
pub fn verify_credential_cose(
    bytes: &[u8],
    key: &PublicKey,
    options: VerifyOptions<'_>,
) -> Result<Value, JoseError> {
    let (payload, iss) = open(bytes, key, options, CREDENTIAL_COSE_TYP, CREDENTIAL_CTY)?;
    if let Some(iss) = iss.as_deref() {
        check_issuer_claim(&payload, iss)?;
    }
    check_credential_properties(&payload)?;
    Ok(Value::Object(payload))
}

/// Verify a presentation secured as a `vp+cose` (VC-JOSE-COSE §3.3.2), and check the envelope of
/// every credential it envelops.
///
/// As [`verify_presentation`](super::verify_presentation) is to a `vp+jwt`: the presentation's
/// own message is verified as [`verify_credential_cose`] verifies a credential's, with `typ`
/// naming `vp+cose` and the content type `vp`, then the presentation's `holder` and `type`, then
/// each envelope. An envelope is judged as an envelope only: its `type`, `@context` and `id`.
/// No credential is verified: §5.3 defines verifying a presentation secured with COSE as the
/// `COSE_Sign1` process on the presentation itself, and §5 says key discovery is done for each
/// document before verification. Use [`VerifiedPresentation::verify_credentials`] or
/// [`UnverifiedEnvelope::verify`](super::UnverifiedEnvelope::verify).
///
/// `iss` is read from the CWT claims as for a credential, and, as for a `vp+jwt`, is not
/// compared with the holder.
///
/// # Errors
///
/// A [`JoseError`] naming the first check that failed.
pub fn verify_presentation_cose(
    bytes: &[u8],
    holder_key: &PublicKey,
    options: VerifyOptions<'_>,
) -> Result<VerifiedPresentation, JoseError> {
    let (payload, _iss) = open(
        bytes,
        holder_key,
        options,
        PRESENTATION_COSE_TYP,
        PRESENTATION_CTY,
    )?;
    finish_presentation(payload)
}

/// Everything about a message that does not depend on whether it carries a credential or a
/// presentation, in the order [`verify_credential_cose`] lists it up to its `iss`: the
/// message, `typ` and the content type, the payload and its `@context`, the time claims and
/// the audience. Returns the payload and the CWT `iss`.
fn open(
    bytes: &[u8],
    key: &PublicKey,
    options: VerifyOptions<'_>,
    typ: &'static str,
    cty: &'static str,
) -> Result<(Map<String, Value>, Option<String>), JoseError> {
    let VerifiedSign1 {
        payload,
        typ: found_typ,
        content_type,
        cwt_claims,
        ..
    } = verify_sign1(bytes, key)?;
    check_media_types(found_typ.as_ref(), content_type.as_ref(), typ, cty)?;

    let Value::Object(payload) =
        serde_json::from_slice(&payload).map_err(|e| JoseError::MalformedPayload {
            reason: format!("not JSON: {e}"),
        })?
    else {
        return Err(JoseError::MalformedPayload {
            reason: "not a JSON object".to_owned(),
        });
    };
    check_context(&payload)?;
    let CwtClaims {
        iss, aud, exp, nbf, ..
    } = cwt_claims;
    if let Some(now) = options.now {
        // Exact for every instant a NumericDate can sensibly name, as in the JOSE form.
        let now = now as f64;
        let leeway = options.leeway().as_secs_f64();
        for (claim, seconds) in [("exp", exp), ("nbf", nbf)] {
            if let Some(seconds) = seconds {
                check_instant(claim, seconds, now, leeway)?;
            }
        }
    }
    check_cwt_audience(aud.as_deref(), options.audience)?;
    Ok((payload, iss))
}

fn check_media_types(
    typ: Option<&dpp_crypto::cose::ContentType>,
    content_type: Option<&dpp_crypto::cose::ContentType>,
    expected_typ: &'static str,
    expected_cty: &'static str,
) -> Result<(), JoseError> {
    match media_type::cose_type_refusal(typ, content_type, expected_typ, expected_cty) {
        None => Ok(()),
        Some(TypeRefusal::Typ { found }) => Err(JoseError::WrongMediaType {
            member: "typ",
            found,
            expected: expected_typ,
        }),
        Some(TypeRefusal::Cty { found }) => Err(JoseError::WrongMediaType {
            member: "cty",
            found: Some(found),
            expected: expected_cty,
        }),
        // A COSE header has no unreadable form here: the COSE layer has already read it.
        Some(TypeRefusal::Unreadable) => Err(JoseError::Cose(CoseError::Malformed(
            "the protected header",
        ))),
    }
}
