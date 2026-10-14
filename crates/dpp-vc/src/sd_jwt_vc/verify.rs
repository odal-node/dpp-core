//! Verification — signature first, then the profile's claims, then disclosures.

use chrono::{DateTime, Utc};
use serde_json::{Map, Value};

use dpp_crypto::sd_jwt::{KeyBindingPolicy, SdJwt};

use super::TYP;
use super::error::SdJwtVcError;

/// The temporal claims of draft-ietf-oauth-sd-jwt-vc-19 clause 2.2.2, each
/// OPTIONAL, each a JWT `NumericDate` (RFC 7519 clause 2) when present.
const TEMPORAL_CLAIMS: [&str; 3] = ["iat", "nbf", "exp"];

/// The registered claims clause 2.2.2.3 says MUST NOT be included in Disclosures,
/// sub-claims included.
const NEVER_DISCLOSED: [&str; 8] = [
    "iss",
    "nbf",
    "exp",
    "cnf",
    "vct",
    "vct#integrity",
    "aka_vcts",
    "status",
];

/// Verify a credential or a presentation, and return what it discloses.
///
/// `public_key_b64` is the issuer's verification key, base64url-encoded, which a
/// consumer obtains from the document
/// [`build_issuer_metadata`](super::build_issuer_metadata) produces by matching
/// the JWT's `kid`. Fetching that document is I/O and therefore not this crate's
/// job — the same reason `did:web` resolution has always lived outside it.
///
/// Checks, in order: the issuer's signature over the token; `typ`; the
/// disclosure mechanism, which refuses any disclosure that matches nothing; Key
/// Binding, as `key_binding` asks; that no claim clause 2.2.2.3 keeps out of
/// Disclosures arrived in one; that a `status`, if present, is an object; the
/// presence of `iss` and `vct`; `vct` against what the caller expected; and the
/// validity window.
///
/// What comes back is every claim of the credential. A credential [`issue`](super::issue)
/// made holds its passport under [`PASSPORT_CLAIM`](super::PASSPORT_CLAIM) and
/// nothing of the passport at the root, so the root names are the profile's.
///
/// # Key Binding
///
/// `key_binding` is the caller's policy, and never the token's shape (RFC 9901
/// clause 7.3 step 1):
///
/// - **`Some(policy)`** requires a Key Binding JWT, signed by the key the
///   credential's `cnf` names, for the policy's audience and nonce, and with an
///   `iat` inside its window (clause 7.3 step 5, and the profile's clause 2.4).
///   The policy carries its own instant, which should be `now`.
/// - **`None`** expects an SD-JWT, and refuses a token that carries a Key Binding
///   JWT: clause 4 says a Verifier that expects an SD-JWT MUST find the last
///   segment empty.
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
    key_binding: Option<&KeyBindingPolicy>,
) -> Result<Map<String, Value>, SdJwtVcError> {
    let sd_jwt = SdJwt::parse(serialised)?;

    match dpp_crypto::jws::verifier::verify_jws(sd_jwt.jwt(), public_key_b64) {
        Ok(true) => {}
        _ => return Err(SdJwtVcError::BadSignature),
    }

    let typ = jwt_part(sd_jwt.jwt(), 0)
        .and_then(|header| header.get("typ")?.as_str().map(str::to_owned))
        .unwrap_or_default();
    if typ != TYP {
        return Err(SdJwtVcError::WrongTyp(typ));
    }

    let payload = match key_binding {
        None if sd_jwt.has_key_binding() => return Err(SdJwtVcError::UnexpectedKeyBinding),
        None => sd_jwt.disclosed_payload()?,
        Some(policy) => sd_jwt.disclosed_payload_with_key_binding(policy)?,
    };

    // What the Issuer signed in the clear is the claim. Anything a Disclosure
    // added or changed underneath one of these names makes the processed value
    // differ from it. A Disclosure the Holder withheld cannot be seen here: keeping
    // these claims out of Disclosures is the Issuer's half of the rule.
    let signed = jwt_part(sd_jwt.jwt(), 1).unwrap_or_default();
    if let Some(claim) = NEVER_DISCLOSED
        .into_iter()
        .find(|claim| payload.get(*claim) != signed.get(*claim))
    {
        return Err(SdJwtVcError::DisclosedClaim(claim));
    }

    // The profile registers `status` for a reference to a Token Status List
    // (clause 2.2.2.3), whose value is an object (draft-ietf-oauth-status-list-21
    // section 6.2). The reference is not followed, so the shape is all that can be
    // held to. Without it a string, which is what a passport's own lifecycle
    // `status` is, would pass for the claim a Verifier is told to check.
    if payload
        .get("status")
        .is_some_and(|status| !status.is_object())
    {
        return Err(SdJwtVcError::MalformedStatus);
    }

    // Presence is not enough: `contains_key` accepts `"iss": null`, a number or
    // an array, and the credential would then verify while naming no issuer at
    // all. Checked the way `vct` below is checked, because a claim that has to
    // be *read* has to be a string.
    if payload.get("iss").and_then(Value::as_str).is_none() {
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
        //
        // The clause also says so in as many words of non-integer values, which
        // `as_i64` refused: a conformant `1789003600.5` was read as malformed
        // and the credential rejected. Refusing a valid credential is the
        // failure here, so the number is taken as `f64` and its fraction kept.
        // Everything that is *not* a finite number stays refused, including a
        // value too large to land on the timeline — the fail-closed direction
        // below only holds while a malformed claim cannot read as an absent one.
        let Some(seconds) = value.as_f64().filter(|s| s.is_finite()) else {
            return Err(SdJwtVcError::MalformedTemporalClaim(claim));
        };
        let whole = seconds.floor();
        #[expect(
            clippy::cast_possible_truncation,
            reason = "guarded: `whole` is finite and range-checked by from_timestamp below"
        )]
        let nanos = ((seconds - whole) * 1e9).round() as u32;
        #[expect(
            clippy::cast_possible_truncation,
            reason = "same guard; an out-of-range i64 cast is caught by from_timestamp"
        )]
        let Some(instant) = DateTime::from_timestamp(whole as i64, nanos.min(999_999_999)) else {
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

/// Part `index` of a compact JWS, the protected header (0) or the payload (1), as
/// a JSON object.
fn jwt_part(jwt: &str, index: usize) -> Option<Map<String, Value>> {
    use base64::Engine;
    let part = jwt.split('.').nth(index)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .ok()?;
    match serde_json::from_slice(&bytes).ok()? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}
