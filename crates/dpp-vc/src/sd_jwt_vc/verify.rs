//! Verification — signature first, then the profile's claims, then disclosures.

use serde_json::{Map, Value};

use dpp_crypto::sd_jwt::SdJwt;

use super::TYP;
use super::error::SdJwtVcError;

/// Verify a credential or a presentation, and return what it discloses.
///
/// `public_key_b64` is the issuer's verification key, base64url-encoded, which a
/// consumer obtains from the document
/// [`build_issuer_metadata`](super::build_issuer_metadata) produces by matching
/// the JWT's `kid`. Fetching that document is I/O and therefore not this crate's
/// job — the same reason `did:web` resolution has always lived outside it.
///
/// Checks, in order: the issuer's signature over the token; `typ`; the presence
/// of `iss` and `vct`; `vct` against what the caller expected; and finally the
/// disclosure mechanism, which refuses any disclosure that matches nothing.
///
/// **Order matters.** The signature is checked first, so nothing downstream ever
/// reasons about claims from an unauthenticated token.
///
/// # Errors
///
/// See [`SdJwtVcError`]. A tampered disclosure surfaces as
/// [`SdJwtError::UnusedDisclosures`](dpp_crypto::sd_jwt::SdJwtError::UnusedDisclosures),
/// not as a missing claim — the credential is refused rather than silently
/// losing the claim that was altered.
pub fn verify(
    serialised: &str,
    public_key_b64: &str,
    expected_vct: &str,
) -> Result<Map<String, Value>, SdJwtVcError> {
    let sd_jwt = SdJwt::parse(serialised)?;

    match dpp_crypto::jws::verifier::verify_jws(sd_jwt.jwt(), public_key_b64) {
        Ok(true) => {}
        _ => return Err(SdJwtVcError::BadSignature),
    }

    let typ = jwt_header_typ(sd_jwt.jwt()).unwrap_or_default();
    if typ != TYP {
        return Err(SdJwtVcError::WrongTyp(typ));
    }

    let payload = sd_jwt.disclosed_payload()?;

    if !payload.contains_key("iss") {
        return Err(SdJwtVcError::MissingClaim("iss"));
    }
    let Some(found) = payload.get("vct").and_then(Value::as_str) else {
        return Err(SdJwtVcError::MissingClaim("vct"));
    };
    if found != expected_vct {
        return Err(SdJwtVcError::UnexpectedVct {
            expected: expected_vct.to_owned(),
            found: found.to_owned(),
        });
    }

    Ok(payload)
}

/// The `typ` from a compact JWS protected header.
fn jwt_header_typ(jwt: &str) -> Option<String> {
    use base64::Engine;
    let header_b64 = jwt.split('.').next()?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(header_b64)
        .ok()?;
    let header: Value = serde_json::from_slice(&bytes).ok()?;
    header.get("typ")?.as_str().map(str::to_owned)
}
