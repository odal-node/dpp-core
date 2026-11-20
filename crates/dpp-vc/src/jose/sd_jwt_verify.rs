//! Verifying a credential or a presentation secured as a VC-JOSE-COSE SD-JWT.

use serde_json::Value;

use dpp_crypto::jws::PublicKey;
use dpp_crypto::sd_jwt::SdJwt;

use super::error::JoseError;
use super::presentation::finish_presentation;
use super::properties::check_credential_properties;
use super::verified_presentation::VerifiedPresentation;
use super::verify::{
    CREDENTIAL_CTY, Opened, PRESENTATION_CTY, check_issuer, claims_of, open_token,
};

/// The `typ` of a credential secured as an SD-JWT, and of a presentation (VC-JOSE-COSE
/// §3.2.1 and §3.2.2). The `cty` is the one of the `vc+jwt` form.
pub(crate) const CREDENTIAL_SD_JWT_TYP: &str = "vc+sd-jwt";
pub(crate) const PRESENTATION_SD_JWT_TYP: &str = "vp+sd-jwt";

/// The claims RFC 9901 clause 9.7 says an Issuer must not make selectively disclosable:
/// "critical for evaluating the SD-JWT's authenticity or validity". `iss`, `aud`, `exp`,
/// `nbf` and `cnf` are the ones it names.
pub(crate) const NEVER_DISCLOSED: [&str; 5] = ["iss", "aud", "exp", "nbf", "cnf"];

/// Verify a credential secured as a `vc+sd-jwt` (VC-JOSE-COSE §3.2.1), and return the
/// credential it carries, as far as it was disclosed.
///
/// `sd_jwt` is the combined format of RFC 9901 clause 4, `<JWT>~<Disclosure>~...~`. `key`
/// and `now` are as for [`verify_credential`](super::verify_credential), and so is every
/// check after the Disclosures are applied. Refusing at the first that fails, the order is:
///
/// 1. the combined format ([`JoseError::MalformedSdJwt`]);
/// 2. the Issuer-signed JWT: compact form, protected header, signature, `typ` and `cty`,
///    exactly as for a `vc+jwt`, with `typ` naming `vc+sd-jwt`;
/// 3. no Key Binding JWT ([`JoseError::KeyBindingUnsupported`]);
/// 4. the Disclosures, processed as RFC 9901 clause 7.1 says, which refuses any Disclosure
///    that matches no digest ([`JoseError::InvalidDisclosures`]), and that none of `iss`,
///    `aud`, `exp`, `nbf` and `cnf` came from one ([`JoseError::DisclosedCriticalClaim`],
///    RFC 9901 clause 9.7);
/// 5. on the processed payload, which is the JWT Claims Set VC-JOSE-COSE §5.2 says to
///    convert it back to: `@context`, `vc` and `vp`, the time claims, `iss` and the `kid`,
///    and the credential's `type`, `issuer` and `credentialSubject`, as for a `vc+jwt`.
///
/// What comes back holds the claims the Disclosures supplied, and lacks every claim the
/// holder did not disclose. A credential whose `credentialSubject` was concealed and not
/// disclosed is refused as one with no subject, since the checks read what is there.
///
/// # Key Binding
///
/// A token that carries a Key Binding JWT is refused. This function takes no Key Binding
/// policy, which RFC 9901 clause 7.3 says is the verifier's to choose: an audience, a
/// nonce and a window. The suite's inputs carry none.
///
/// # Errors
///
/// A [`JoseError`] naming the first check the token failed.
pub fn verify_credential_sd_jwt(
    sd_jwt: &str,
    key: &PublicKey,
    now: Option<i64>,
) -> Result<Value, JoseError> {
    let Opened { payload, kid } =
        open_sd_jwt(sd_jwt, key, now, CREDENTIAL_SD_JWT_TYP, CREDENTIAL_CTY)?;
    check_issuer(&payload, kid.as_deref())?;
    check_credential_properties(&payload)?;
    Ok(Value::Object(payload))
}

/// Verify a presentation secured as a `vp+sd-jwt` (VC-JOSE-COSE §3.2.2), and check the
/// shape of every credential it envelops.
///
/// As [`verify_presentation`](super::verify_presentation) is to a `vp+jwt`: the
/// presentation's own SD-JWT is verified as [`verify_credential_sd_jwt`] verifies a
/// credential's, with `typ` naming `vp+sd-jwt`, then the presentation's `holder`, `kid` and
/// `type`, then each envelope's shape. A `vc+jwt` or `vc+sd-jwt` envelope's data must be
/// readable as one. No envelope is verified: use
/// [`VerifiedPresentation::verify_credentials`] or [`UnverifiedEnvelope::verify`](super::UnverifiedEnvelope::verify).
///
/// # Errors
///
/// A [`JoseError`] naming the first check that failed.
pub fn verify_presentation_sd_jwt(
    sd_jwt: &str,
    holder_key: &PublicKey,
    now: Option<i64>,
) -> Result<VerifiedPresentation, JoseError> {
    let Opened { payload, kid } = open_sd_jwt(
        sd_jwt,
        holder_key,
        now,
        PRESENTATION_SD_JWT_TYP,
        PRESENTATION_CTY,
    )?;
    finish_presentation(payload, kid.as_deref())
}

/// Steps 1 to 5 of [`verify_credential_sd_jwt`] up to the claims checks that follow in the
/// caller: the SD-JWT as a token, then its processed payload through the checks every
/// claims set meets.
fn open_sd_jwt(
    sd_jwt: &str,
    key: &PublicKey,
    now: Option<i64>,
    typ: &'static str,
    cty: &'static str,
) -> Result<Opened, JoseError> {
    let parsed = SdJwt::parse(sd_jwt).map_err(|e| JoseError::MalformedSdJwt {
        reason: e.to_string(),
    })?;
    let signed = open_token(parsed.jwt(), key, typ, cty)?;
    if parsed.has_key_binding() {
        return Err(JoseError::KeyBindingUnsupported);
    }
    let processed = parsed
        .disclosed_payload()
        .map_err(|e| JoseError::InvalidDisclosures {
            reason: e.to_string(),
        })?;
    if let Some(name) = NEVER_DISCLOSED
        .into_iter()
        .find(|name| processed.get(*name) != signed.get(*name))
    {
        return Err(JoseError::DisclosedCriticalClaim { name });
    }
    claims_of(processed, parsed.jwt(), now)
}
