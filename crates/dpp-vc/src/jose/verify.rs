//! Verifying a credential secured as a VC-JOSE-COSE `vc+jwt`, and the checks a
//! presentation's outer token shares with it.

use base64::Engine as _;
use serde_json::{Map, Value};

use dpp_crypto::jws::verifier::{extract_kid_from_jws, header_refusal, verify_jws};
use dpp_crypto::jws::{HeaderRefusal, PublicKey};

use super::absolute_url::is_absolute_url;
use super::error::JoseError;
use super::properties::{check_context, check_credential_properties, id_of};
use crate::common::media_type::{self, TypeRefusal};

/// The `typ` of a secured credential, and the `cty` naming what it carries
/// (VC-JOSE-COSE §3.1.1).
pub(crate) const CREDENTIAL_TYP: &str = "vc+jwt";
pub(crate) const CREDENTIAL_CTY: &str = "vc";

/// The `typ` of a secured presentation, and its `cty` (VC-JOSE-COSE §3.1.2).
pub(crate) const PRESENTATION_TYP: &str = "vp+jwt";
pub(crate) const PRESENTATION_CTY: &str = "vp";

/// A token that passed every check of its own, before anything is read out of it:
/// the claims set, and the `kid` its protected header names.
pub(crate) struct Opened {
    pub(crate) payload: Map<String, Value>,
    pub(crate) kid: Option<String>,
}

/// Verify a credential secured as a `vc+jwt` (VC-JOSE-COSE §3.1.1), and return the
/// credential it carries.
///
/// `key` is the issuer's verification key with its algorithm, which the caller
/// resolved from `kid` by whatever means it trusts: the `kid` is a hint
/// (§4.1.1), never a selector, and this crate performs no network access.
///
/// Checks, in order, refusing at the first that fails:
///
/// 1. the token is a compact JWS ([`JoseError::NotCompactJws`]);
/// 2. the protected header is readable, carries no `crit`, does not set `b64` to
///    `false` (RFC 7797 §7, which [`JoseError::UnencodedPayload`] reports; a header
///    with both `crit` and `b64` is refused for `crit`), and its `alg` is `key`'s
///    algorithm, so `alg: none` and every substitution are refused (§1.1.2.1,
///    RFC 7519 §7.2);
/// 3. the signature verifies under `key`;
/// 4. `typ` is present and names `vc+jwt`, and `cty`, if present, names `vc`
///    (RFC 7515 §4.1.9, with or without the `application/` prefix);
/// 5. the payload is a JSON object whose `@context` is the VC Data Model 2.0 context
///    set (VC DM 2.0 §4.3), and carries neither `vc` nor `vp` (§1.1.2.1, §3.1.3);
/// 6. under `Some(now)`, the time claims, below;
/// 7. `iss`, if present, equals the credential's `issuer`, or its `id` when the
///    issuer is an object (§4.1.2); and when `iss` is absent and the issuer is a
///    URL, the `kid` is an absolute URL (§4.2). That the `kid` names a verification
///    method of the issuer is the caller's, since this function has no identifier
///    document to look it up in;
/// 8. the credential's `type` names `VerifiableCredential`, its `issuer` is a URL or
///    an object with a URL `id`, and its `credentialSubject` is an object or a
///    non-empty array of objects (VC DM 2.0 §4.5, §4.7, §4.8).
///
/// Members this function does not understand are ignored (§3.1.3, §5.4).
///
/// # Time
///
/// `now` is seconds since the Unix epoch, supplied rather than read, as in
/// [`sd_jwt_vc::verify`](crate::sd_jwt_vc::verify): a result that depends on a
/// hidden clock cannot be reproduced.
///
/// - **`Some(now)`**: `exp`, if present, must be a JSON number and `now` must be
///   before it (RFC 7519 §4.1.4); `nbf`, if present, must be a JSON number and
///   `now` must not be before it (RFC 7519 §4.1.5). A value that is not a number is
///   refused, not skipped. There is no leeway: the comparison is exact, as in
///   `sd_jwt_vc::verify`.
/// - **`None`**: no check of `exp` or `nbf`, and not their type either.
///
/// `iat` is never checked, in either case. VC-JOSE-COSE §3.1.3 says `iat` and
/// `exp` describe the signature, not the credential, and the suite's credentials
/// carry `iat` as an RFC 3339 string. The credential's own validity period, its
/// `validFrom` and `validUntil`, is the caller's to check.
///
/// # Errors
///
/// A [`JoseError`] naming the first check the token failed.
pub fn verify_credential(jws: &str, key: &PublicKey, now: Option<i64>) -> Result<Value, JoseError> {
    let Opened { payload, kid } = open(jws, key, now, CREDENTIAL_TYP, CREDENTIAL_CTY)?;
    check_issuer(&payload, kid.as_deref())?;
    check_credential_properties(&payload)?;
    Ok(Value::Object(payload))
}

/// Everything about a token that does not depend on whether it carries a credential
/// or a presentation, in the order [`verify_credential`] lists: the compact form,
/// the header, the signature, `typ` and `cty`, the payload and its `@context`,
/// `vc` and `vp`, and the time claims.
pub(crate) fn open(
    jws: &str,
    key: &PublicKey,
    now: Option<i64>,
    typ: &'static str,
    cty: &'static str,
) -> Result<Opened, JoseError> {
    if jws.split('.').count() != 3 {
        return Err(JoseError::NotCompactJws);
    }
    match header_refusal(jws, key.algorithm()) {
        None => {}
        Some(HeaderRefusal::Critical) => return Err(JoseError::UnsupportedCritical),
        Some(HeaderRefusal::Algorithm { found }) => {
            return Err(JoseError::AlgorithmMismatch {
                header_alg: found,
                key_alg: key.algorithm(),
            });
        }
        Some(_) => return Err(JoseError::UnreadableHeader),
    }
    if unencoded_payload(jws) {
        return Err(JoseError::UnencodedPayload);
    }
    match verify_jws(jws, key) {
        Ok(true) => {}
        Ok(false) => return Err(JoseError::SignatureDoesNotVerify),
        Err(e) => {
            return Err(JoseError::Unverifiable {
                reason: e.to_string(),
            });
        }
    }
    match media_type::header_type_refusal(jws, typ, cty) {
        None => {}
        Some(TypeRefusal::Typ { found }) => {
            return Err(JoseError::WrongMediaType {
                member: "typ",
                found,
                expected: typ,
            });
        }
        Some(TypeRefusal::Cty { found }) => {
            return Err(JoseError::WrongMediaType {
                member: "cty",
                found: Some(found),
                expected: cty,
            });
        }
        Some(TypeRefusal::Unreadable) => return Err(JoseError::UnreadableHeader),
    }

    let payload = jws.split('.').nth(1).unwrap_or_default();
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .map_err(|e| JoseError::MalformedPayload {
            reason: format!("not base64url: {e}"),
        })?;
    let Value::Object(payload) =
        serde_json::from_slice(&bytes).map_err(|e| JoseError::MalformedPayload {
            reason: format!("not JSON: {e}"),
        })?
    else {
        return Err(JoseError::MalformedPayload {
            reason: "not a JSON object".to_owned(),
        });
    };
    check_context(&payload)?;
    for name in ["vc", "vp"] {
        if payload.contains_key(name) {
            return Err(JoseError::ReservedClaim { name });
        }
    }
    if let Some(now) = now {
        check_time(&payload, now)?;
    }
    Ok(Opened {
        payload,
        kid: extract_kid_from_jws(jws),
    })
}

/// Whether the protected header sets `b64` to the boolean `false` (RFC 7797 §7). The
/// header has already been read by [`header_refusal`], so an unreadable one answers
/// `false` here and is refused there.
fn unencoded_payload(jws: &str) -> bool {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    jws.split('.')
        .next()
        .and_then(|segment| b64.decode(segment).ok())
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|header| header.get("b64").cloned())
        == Some(Value::Bool(false))
}

/// `claim` read as a NumericDate: `None` when it is absent, its value when it is a finite
/// JSON number, and [`JoseError::MalformedTemporalClaim`] when it is present and is not one.
fn numeric_date(
    payload: &Map<String, Value>,
    claim: &'static str,
) -> Result<Option<f64>, JoseError> {
    match payload.get(claim) {
        None => Ok(None),
        Some(value) => value
            .as_f64()
            .filter(|s| s.is_finite())
            .map(Some)
            .ok_or(JoseError::MalformedTemporalClaim { claim }),
    }
}

/// `exp` and `nbf`, when present, must be JSON numbers. Verification refuses one that is
/// not, under a `now`, so signing refuses it too; no time is compared here.
pub(crate) fn check_temporal_claims(payload: &Map<String, Value>) -> Result<(), JoseError> {
    for claim in ["exp", "nbf"] {
        numeric_date(payload, claim)?;
    }
    Ok(())
}

/// Enforce `exp` and `nbf` against `now`, and refuse one that is not a number.
///
/// A malformed temporal claim is refused rather than ignored, the same choice
/// `sd_jwt_vc::verify` makes: skipping an `exp` that does not parse would turn it
/// into an absent `exp`, which is the unbounded case. A NumericDate may be
/// fractional (RFC 7519 §2), so the claim is read as a number and compared as one.
fn check_time(payload: &Map<String, Value>, now: i64) -> Result<(), JoseError> {
    // Exact for every instant a NumericDate can sensibly name; an `i64` that does
    // not fit an `f64`'s 53 bits is far past the year 285 million.
    let now = now as f64;
    for claim in ["exp", "nbf"] {
        let Some(seconds) = numeric_date(payload, claim)? else {
            continue;
        };
        match claim {
            "exp" if now >= seconds => return Err(JoseError::Expired),
            "nbf" if now < seconds => return Err(JoseError::NotYetValid),
            _ => {}
        }
    }
    Ok(())
}

/// The issuer of a credential: its `issuer` when that is a string, or its `id` when
/// that is an object (VC-JOSE-COSE §4.1.2).
fn issuer_of(credential: &Map<String, Value>) -> Option<&str> {
    credential.get("issuer").and_then(id_of)
}

/// The holder of a presentation, read as the issuer of a credential is: its `holder`
/// when that is a string, or its `id` when that is an object (VC-JOSE-COSE §4.2).
fn holder_of(presentation: &Map<String, Value>) -> Option<&str> {
    presentation.get("holder").and_then(id_of)
}

/// `iss` against the credential's issuer (§4.1.2), and, when there is no `iss`,
/// [`check_kid`]. Signing runs this too, so a credential whose `iss` names another
/// issuer is not signed.
pub(crate) fn check_issuer(
    credential: &Map<String, Value>,
    kid: Option<&str>,
) -> Result<(), JoseError> {
    let issuer = issuer_of(credential);
    match credential.get("iss") {
        Some(iss) => {
            let Some(iss) = iss.as_str() else {
                return Err(JoseError::MalformedPayload {
                    reason: "iss is not a string".to_owned(),
                });
            };
            if Some(iss) != issuer {
                return Err(JoseError::IssuerMismatch {
                    iss: iss.to_owned(),
                    issuer: issuer.map(str::to_owned),
                });
            }
        }
        None => check_kid(credential, kid)?,
    }
    Ok(())
}

/// When a credential has no `iss` and its issuer is identified by a URL, the `kid`
/// must be an absolute URL (VC-JOSE-COSE §4.2), since nothing else says where the
/// issuer's key is to be found. A DID, an `https` URL and a URN are all absolute
/// URLs here, and a bare key thumbprint is not.
///
/// Only the absolute form is checked. That the URL names a verification method
/// listed in the issuer's identifier document needs the document, and this crate
/// fetches none.
pub(crate) fn check_kid(
    credential: &Map<String, Value>,
    kid: Option<&str>,
) -> Result<(), JoseError> {
    check_kid_of(credential, issuer_of(credential), kid)
}

/// The same rule for a presentation: when it has no `iss` and its holder is identified
/// by a URL, the `kid` must be an absolute URL (VC-JOSE-COSE §4.2, the second rule).
pub(crate) fn check_holder_kid(
    presentation: &Map<String, Value>,
    kid: Option<&str>,
) -> Result<(), JoseError> {
    check_kid_of(presentation, holder_of(presentation), kid)
}

/// The `kid` rule of §4.2 for the party a token names, which is the issuer of a
/// credential or the holder of a presentation. An `iss` that is present must be a string
/// (RFC 7519 §4.1.1), and an `iss` of any other type is refused before the rule is read,
/// so a `null` or a number cannot switch the rule off.
fn check_kid_of(
    document: &Map<String, Value>,
    party: Option<&str>,
    kid: Option<&str>,
) -> Result<(), JoseError> {
    match document.get("iss") {
        Some(iss) if !iss.is_string() => {
            return Err(JoseError::MalformedPayload {
                reason: "iss is not a string".to_owned(),
            });
        }
        Some(_) => return Ok(()),
        None => {}
    }
    if party.is_some_and(is_absolute_url) && !kid.is_some_and(is_absolute_url) {
        return Err(JoseError::KidNotAbsolute {
            kid: kid.map(str::to_owned),
        });
    }
    Ok(())
}
