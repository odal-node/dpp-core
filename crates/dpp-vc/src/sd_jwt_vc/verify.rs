//! Verification — signature first, then the profile's claims, then disclosures.

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use dpp_crypto::sd_jwt::SdJwt;

use super::TYP;
use super::error::SdJwtVcError;

/// The temporal claims of draft-ietf-oauth-sd-jwt-vc-19 clause 2.2.2, each
/// OPTIONAL, each a JWT `NumericDate` (RFC 7519 clause 2) when present.
const TEMPORAL_CLAIMS: [&str; 3] = ["iat", "nbf", "exp"];

/// Verify a credential or a presentation, and return what it discloses.
///
/// `public_key_b64` is the issuer's verification key, base64url-encoded, which a
/// consumer obtains from the document
/// [`build_issuer_metadata`](super::build_issuer_metadata) produces by matching
/// the JWT's `kid`. Fetching that document is I/O and therefore not this crate's
/// job — the same reason `did:web` resolution has always lived outside it.
///
/// Checks, in order: the issuer's signature over the token; `typ`; the
/// disclosure mechanism, which refuses any disclosure that matches nothing; the
/// presence of `iss` and `vct`; `vct` against what the caller expected; and the
/// validity window.
///
/// **Order matters.** The signature is checked first, so nothing downstream ever
/// reasons about claims from an unauthenticated token.
///
/// # Time
///
/// `now` is supplied rather than read, matching
/// [`verify_snapshot_bound`](crate::snapshot::verify_snapshot_bound): a
/// verification result that depends on a hidden clock cannot be reproduced, and
/// an expiry test that cannot be written for a fixed instant does not get
/// written.
///
/// `nbf` and `exp` are OPTIONAL in the profile, so their absence is valid and
/// means "no bound". Present and passed, or present and not yet reached, is a
/// refusal — otherwise a signed credential replays for ever, which is the whole
/// point of carrying the claims.
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
    now: DateTime<Utc>,
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

    check_validity_window(&payload, now)?;

    Ok(payload)
}

/// Enforce `nbf` and `exp`, and type-check every temporal claim present.
///
/// A malformed temporal claim is refused rather than ignored. Skipping one that
/// does not parse would turn a broken `exp` into an absent `exp`, which is the
/// unbounded case — so the safest-looking branch is the one that grants the
/// most, and it would be reached by malformed input.
fn check_validity_window(
    payload: &Map<String, Value>,
    now: DateTime<Utc>,
) -> Result<(), SdJwtVcError> {
    for claim in TEMPORAL_CLAIMS {
        let Some(value) = payload.get(claim) else {
            continue;
        };
        // RFC 7519 clause 2: a NumericDate is a JSON *number* of seconds since
        // the epoch. A string that looks like a date is not one, and accepting
        // it would read a claim the issuer did not make.
        let Some(seconds) = value.as_i64() else {
            return Err(SdJwtVcError::MalformedTemporalClaim(claim));
        };
        let Some(instant) = DateTime::from_timestamp(seconds, 0) else {
            return Err(SdJwtVcError::MalformedTemporalClaim(claim));
        };
        match claim {
            "exp" if now >= instant => return Err(SdJwtVcError::Expired { at: instant }),
            "nbf" if now < instant => return Err(SdJwtVcError::NotYetValid { from: instant }),
            _ => {}
        }
    }
    Ok(())
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
