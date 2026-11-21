//! Verifying a presentation secured as a VC-JOSE-COSE `vp+jwt`: its own token, and the
//! shape of each credential it envelops.

use serde_json::{Map, Value};

use dpp_crypto::jws::PublicKey;

use super::envelope_shape::inspect;
use super::error::JoseError;
use super::options::VerifyOptions;
use super::properties::{check_holder, check_presentation_properties};
use super::verified_presentation::VerifiedPresentation;
use super::verify::{Opened, PRESENTATION_CTY, PRESENTATION_TYP, check_holder_kid, open};

/// Verify a presentation secured as a `vp+jwt` (VC-JOSE-COSE §3.1.2), and check the shape
/// of every credential it envelops.
///
/// This verifies the presentation's own token and nothing inside an envelope. VC-JOSE-COSE
/// §5.1 says verifying a presentation secured with JOSE means following the algorithm
/// "Validating a JWT" of RFC 7519, and §5 says key discovery "is assumed to have succeeded
/// prior to beginning the verification process". The credentials are verified separately, with
/// [`VerifiedPresentation::verify_credentials`] or one at a time with
/// [`UnverifiedEnvelope::verify`](super::UnverifiedEnvelope::verify), each under its own key.
///
/// `holder_key` is the holder's verification key for the outer token, with the same meaning
/// as in [`verify_credential`](super::verify_credential). The checks run in this order,
/// refusing at the first that fails: the outer token (compact form, header,
/// signature, `typ` and `cty`, payload and its `@context`, `vc` and `vp`, the time
/// claims under a `now`, and the audience), the presentation's `holder`, when present, which must be a
/// URL or an object with a URL `id` (VC DM 2.0 §4.13), the holder's `kid` (§4.2), the
/// presentation's `type` (VC DM 2.0 §4.13), and then each envelope in order.
///
/// `options` is as for [`verify_credential`](super::verify_credential): the `now` and the
/// audience apply to the presentation's own token, and a presentation whose `aud` does not name
/// the audience is refused. The credentials it envelops are checked, later, against the options
/// given there.
///
/// Each member of `verifiableCredential` must be an Enveloped Verifiable Credential (VC DM
/// 2.0 §4.13): an object whose `type` is `EnvelopedVerifiableCredential`, alone or as a
/// one-item array, with an `@context`, and an `id` that is a `data:` URL. A `vc+jwt`
/// envelope's data must also be a compact JWS, and a `vc+sd-jwt` envelope's an SD-JWT. Any
/// envelope that fails these refuses the presentation with [`JoseError::MalformedEnvelope`].
///
/// # Errors
///
/// A [`JoseError`] naming the first check that failed.
pub fn verify_presentation(
    jws: &str,
    holder_key: &PublicKey,
    options: VerifyOptions<'_>,
) -> Result<VerifiedPresentation, JoseError> {
    let Opened { payload, kid } =
        open(jws, holder_key, options, PRESENTATION_TYP, PRESENTATION_CTY)?;
    finish_presentation(payload, kid.as_deref())
}

/// What follows the outer token in every secured form: the presentation's `holder`, the
/// holder's `kid`, its `type`, and the shape of each envelope. `payload` is the claims set
/// of a `vp+jwt`, or an SD-JWT's once its Disclosures are applied.
pub(crate) fn finish_presentation(
    payload: Map<String, Value>,
    kid: Option<&str>,
) -> Result<VerifiedPresentation, JoseError> {
    check_holder(&payload)?;
    check_holder_kid(&payload, kid)?;
    check_presentation_properties(&payload)?;

    // JSON-LD lets a property that holds one value hold it without the array.
    let members: Vec<&Value> = match payload.get("verifiableCredential") {
        None => Vec::new(),
        Some(Value::Array(members)) => members.iter().collect(),
        Some(single) => vec![single],
    };
    let envelopes = members
        .into_iter()
        .enumerate()
        .map(|(index, member)| inspect(index, member))
        .collect::<Result<Vec<_>, _>>()?;

    Ok(VerifiedPresentation::new(Value::Object(payload), envelopes))
}
