//! Verifying a presentation secured as a VC-JOSE-COSE `vp+jwt`.

use serde_json::Value;

use dpp_crypto::jws::PublicKey;

use super::envelope_decision::EnvelopeDecision;
use super::envelope_shape::inspect;
use super::error::JoseError;
use super::skipped_envelope::SkippedEnvelope;
use super::verified_envelope::VerifiedEnvelope;
use super::verified_presentation::VerifiedPresentation;
use super::verify::{PRESENTATION_CTY, PRESENTATION_TYP, open, verify_credential};

/// Verify a presentation secured as a `vp+jwt` (VC-JOSE-COSE §3.1.2), and account
/// for every credential it envelops.
///
/// `key` is the holder's verification key for the outer token, with the same
/// meaning as in [`verify_credential`]. The outer token is checked completely
/// first, signature, header, `typ` (`vp+jwt`, with `cty` `vp` if present), claims
/// and time, before any envelope is read: §5 says a verifier should minimise the
/// processing of untrusted data. No `holder` against `iss` comparison is made.
///
/// # The envelopes
///
/// Each member of `verifiableCredential` must be an Enveloped Verifiable
/// Credential: an object whose `type` is `EnvelopedVerifiableCredential`, with an
/// `@context`, and an `id` that is a `data:` URL; and when it is a
/// `vc+jwt`, its data must be a compact JWS. Those shape checks come first, for
/// every envelope, and refuse the presentation whatever the caller would answer.
///
/// Then `envelope_key` is called once per envelope, in order, with its index, its
/// media type, and, for a `vc+jwt`, the `kid` its protected header names (`None`
/// when it names none). It answers with an [`EnvelopeDecision`]:
///
/// - **`Verify(key)`** for a `vc+jwt` verifies the inner token with
///   [`verify_credential`], under the same `now`, and any failure refuses the
///   presentation, naming the envelope ([`JoseError::CredentialRefused`]). For any
///   other media type it refuses the presentation: this crate verifies only
///   `vc+jwt` ([`JoseError::UnsupportedEnvelopeFormat`]).
/// - **`Skip`** leaves the envelope unverified and lists it in
///   [`VerifiedPresentation::skipped`], with its index, media type and `kid`.
/// - **`Unknown`** refuses the presentation ([`JoseError::EnvelopeNotDecided`]).
///
/// So a presentation whose envelopes this crate cannot verify, such as a
/// `vc+sd-jwt` or a `vc+cose`, passes only when the caller skips them, and the
/// result says so. Nothing passes unverified without being reported.
///
/// # Errors
///
/// A [`JoseError`] naming the first check that failed.
pub fn verify_presentation<F>(
    jws: &str,
    key: &PublicKey,
    now: Option<i64>,
    mut envelope_key: F,
) -> Result<VerifiedPresentation, JoseError>
where
    F: FnMut(usize, &str, Option<&str>) -> EnvelopeDecision,
{
    let payload = open(jws, key, now, PRESENTATION_TYP, PRESENTATION_CTY)?.payload;

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

    let mut verified = Vec::new();
    let mut skipped = Vec::new();
    for (index, envelope) in envelopes.into_iter().enumerate() {
        let decision = envelope_key(index, &envelope.media_type, envelope.kid.as_deref());
        match (decision, envelope.token) {
            (EnvelopeDecision::Verify(inner_key), Some(token)) => {
                let credential = verify_credential(&token, &inner_key, now).map_err(|source| {
                    JoseError::CredentialRefused {
                        index,
                        source: Box::new(source),
                    }
                })?;
                verified.push(VerifiedEnvelope { index, credential });
            }
            (EnvelopeDecision::Verify(_), None) => {
                return Err(JoseError::UnsupportedEnvelopeFormat {
                    index,
                    media_type: envelope.media_type,
                });
            }
            (EnvelopeDecision::Skip, _) => skipped.push(SkippedEnvelope {
                index,
                media_type: envelope.media_type,
                kid: envelope.kid,
            }),
            (EnvelopeDecision::Unknown, _) => {
                return Err(JoseError::EnvelopeNotDecided {
                    index,
                    media_type: envelope.media_type,
                    kid: envelope.kid,
                });
            }
        }
    }
    Ok(VerifiedPresentation {
        payload: Value::Object(payload),
        verified,
        skipped,
    })
}
