//! Verifying a credential or a presentation secured as a VC-JOSE-COSE SD-JWT.

use serde_json::Value;

use dpp_crypto::jws::PublicKey;
use dpp_crypto::sd_jwt::{SdJwt, SdJwtError};

use super::error::JoseError;
use super::options::{KeyBindingRequirement, VerifyOptions};
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

/// Verify a credential secured as a `vc+sd-jwt` (VC-JOSE-COSE §3.2.1), and return the
/// credential it carries, as far as it was disclosed.
///
/// `sd_jwt` is the combined format of RFC 9901 clause 4, `<JWT>~<Disclosure>~...~`, or that
/// with a Key Binding JWT after the last `~`. `key` and `options` are as for
/// [`verify_credential`](super::verify_credential), and so is every check after the
/// Disclosures are applied, the audience included. `binding` is the choice RFC 9901 §9.5
/// says a verifier makes "before verifying a credential", and it has no default. Refusing at
/// the first check that fails, the order is:
///
/// 1. the combined format ([`JoseError::MalformedSdJwt`]);
/// 2. the Issuer-signed JWT: compact form, protected header, signature, `typ` and `cty`,
///    exactly as for a `vc+jwt`, with `typ` naming `vc+sd-jwt`;
/// 3. the Disclosures, processed as RFC 9901 clause 7.1 says, which refuses any Disclosure
///    that matches no digest ([`JoseError::InvalidDisclosures`]), and, when Key Binding is
///    required, the checks of §7.3 step 5 ([`JoseError::KeyBindingMissing`] and
///    [`JoseError::KeyBindingRefused`]), below;
/// 4. on the processed payload, which is the JWT Claims Set VC-JOSE-COSE §5.2 says to
///    convert it back to: `@context`, `vc` and `vp`, the time claims, the audience, `iss`
///    and the `kid`, and the credential's `type`, `issuer` and `credentialSubject`, as for a
///    `vc+jwt`.
///
/// What comes back holds the claims the Disclosures supplied, and lacks every claim the
/// holder did not disclose. A credential whose `credentialSubject` was concealed and not
/// disclosed is refused as one with no subject, since the checks read what is there.
///
/// # Disclosed validity claims
///
/// A claim that arrived in a Disclosure is checked as one in the clear, because step 4 reads
/// the processed payload: a disclosed `exp` in the past is refused as expired when the
/// options carry a `now`, as one in the clear is, and an `aud` entry that was disclosed is
/// part of the `aud` the audience is compared with. RFC 9901
/// §9.7 puts the duty not to make `iss`, `aud`, `exp`, `nbf` or `cnf` selectively disclosable
/// on the Issuer, and §7.1 step 6 has the verifier check them in the processed payload, so
/// this verifier does not refuse a token for the way a claim travelled.
///
/// # Key Binding
///
/// - **[`KeyBindingRequirement::NotRequired`]**: a Key Binding JWT the token carries is not
///   read, and the token is processed as an SD-JWT (RFC 9901 §7.3 steps 3 and 4). That
///   holds for a token that carries one and for one that does not.
/// - **[`KeyBindingRequirement::Required`]**: the token must carry a Key Binding JWT, and it
///   must satisfy the policy, which is [`SdJwt::disclosed_payload_with_key_binding`]
///   (§7.3 steps 2 and 5), using the Holder key in the `cnf` of the processed payload
///   (RFC 7800 §3.2, RFC 9901 §4.1.2). A token with no `cnf`, or with no Key Binding JWT,
///   is refused.
///
/// # Errors
///
/// A [`JoseError`] naming the first check the token failed.
pub fn verify_credential_sd_jwt(
    sd_jwt: &str,
    key: &PublicKey,
    options: VerifyOptions<'_>,
    binding: &KeyBindingRequirement,
) -> Result<Value, JoseError> {
    let Opened { payload, kid } = open_sd_jwt(
        sd_jwt,
        key,
        options,
        binding,
        CREDENTIAL_SD_JWT_TYP,
        CREDENTIAL_CTY,
    )?;
    check_issuer(&payload, kid.as_deref())?;
    check_credential_properties(&payload)?;
    Ok(Value::Object(payload))
}

/// Verify a presentation secured as a `vp+sd-jwt` (VC-JOSE-COSE §3.2.2), and check the
/// shape of every credential it envelops.
///
/// As [`verify_presentation`](super::verify_presentation) is to a `vp+jwt`: the
/// presentation's own SD-JWT is verified as [`verify_credential_sd_jwt`] verifies a
/// credential's, with `typ` naming `vp+sd-jwt`, `options` and `binding` as it takes them
/// (the Key Binding, when required, is the Holder's over the presentation, with the Holder
/// key in the presentation's own `cnf`), then the presentation's `holder`, `kid` and `type`,
/// then each envelope's shape. A `vc+jwt` or `vc+sd-jwt` envelope's data must be readable as
/// one. No envelope is verified: use
/// [`VerifiedPresentation::verify_credentials`] or [`UnverifiedEnvelope::verify`](super::UnverifiedEnvelope::verify).
///
/// # Errors
///
/// A [`JoseError`] naming the first check that failed.
pub fn verify_presentation_sd_jwt(
    sd_jwt: &str,
    holder_key: &PublicKey,
    options: VerifyOptions<'_>,
    binding: &KeyBindingRequirement,
) -> Result<VerifiedPresentation, JoseError> {
    let Opened { payload, kid } = open_sd_jwt(
        sd_jwt,
        holder_key,
        options,
        binding,
        PRESENTATION_SD_JWT_TYP,
        PRESENTATION_CTY,
    )?;
    finish_presentation(payload, kid.as_deref())
}

/// Steps 1 to 4 of [`verify_credential_sd_jwt`] up to the claims checks that follow in the
/// caller: the SD-JWT as a token, its Disclosures applied, and, when `binding` requires it,
/// its Key Binding JWT checked, and then the processed payload through the checks every
/// claims set meets.
fn open_sd_jwt(
    sd_jwt: &str,
    key: &PublicKey,
    options: VerifyOptions<'_>,
    binding: &KeyBindingRequirement,
    typ: &'static str,
    cty: &'static str,
) -> Result<Opened, JoseError> {
    let parsed = SdJwt::parse(sd_jwt).map_err(|e| JoseError::MalformedSdJwt {
        reason: e.to_string(),
    })?;
    open_token(parsed.jwt(), key, typ, cty)?;
    let processed = match binding {
        KeyBindingRequirement::NotRequired => {
            parsed
                .disclosed_payload()
                .map_err(|e| JoseError::InvalidDisclosures {
                    reason: e.to_string(),
                })?
        }
        KeyBindingRequirement::Required(policy) => parsed
            .disclosed_payload_with_key_binding(policy)
            .map_err(key_binding_refusal)?,
    };
    claims_of(processed, parsed.jwt(), options)
}

/// What [`SdJwt::disclosed_payload_with_key_binding`] refused, as a [`JoseError`]: the
/// Key Binding JWT's absence, the Holder key or the Key Binding JWT, or, for anything else,
/// the Disclosures it processes first.
fn key_binding_refusal(refusal: SdJwtError) -> JoseError {
    match refusal {
        SdJwtError::KeyBindingMissing => JoseError::KeyBindingMissing,
        SdJwtError::NoHolderKey
        | SdJwtError::UnsupportedHolderKey
        | SdJwtError::KeyBindingRejected(_) => JoseError::KeyBindingRefused {
            reason: refusal.to_string(),
        },
        other => JoseError::InvalidDisclosures {
            reason: other.to_string(),
        },
    }
}
