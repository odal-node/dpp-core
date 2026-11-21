//! The shape of an Enveloped Verifiable Credential in a presentation, checked
//! before anything is verified.
//!
//! VC-JOSE-COSE §3.1.2 says the credentials of a presentation MUST use the
//! Enveloped Verifiable Credential type of the VC Data Model, whose section on
//! Enveloped Verifiable Credentials says the object's `@context` MUST be present,
//! its `id` MUST be a `data:` URL, and its `type` MUST be
//! `EnvelopedVerifiableCredential`. §3.2.2 and §3.3.2 say the same of the other two forms.
//!
//! **Only the envelope is judged.** What is in the `data:` URL, a compact JWS, an SD-JWT or a
//! `COSE_Sign1`, is the credential's own, and §5.1 makes verifying a presentation and
//! verifying each credential in it two operations. So the content is not read here. It is
//! read when the credential is verified ([`UnverifiedEnvelope::verify`]), where content that
//! cannot be read is refused and the credential's refusal says why.

use serde_json::Value;

use dpp_crypto::jws::verifier::extract_kid_from_jws;

use super::data_url::{decode_text, split_data_url};
use super::error::JoseError;
use super::unverified_envelope::{EnvelopeData, UnverifiedEnvelope, VC_COSE, VC_JWT, VC_SD_JWT};

/// The `type` of an Enveloped Verifiable Credential object.
const ENVELOPED: &str = "EnvelopedVerifiableCredential";

/// Check the envelope at `index`, and keep what [`verify_credentials`](super::VerifiedPresentation::verify_credentials)
/// needs.
///
/// The `id` is held to being a `data:` URL, and its data is kept for the three media types this
/// crate verifies: `vc+jwt`, `vc+sd-jwt` and `vc+cose`. It is not read: see the module
/// documentation.
pub(crate) fn inspect(index: usize, value: &Value) -> Result<UnverifiedEnvelope, JoseError> {
    let malformed = |reason: &str| JoseError::MalformedEnvelope {
        index,
        reason: reason.to_owned(),
    };
    let Some(object) = value.as_object() else {
        return Err(malformed("not a JSON object"));
    };
    // JSON-LD reads a one-item array as the item itself, so `type` may be either.
    let is_enveloped = match object.get("type") {
        Some(Value::String(name)) => name == ENVELOPED,
        Some(Value::Array(names)) => {
            matches!(names.as_slice(), [Value::String(name)] if name == ENVELOPED)
        }
        _ => false,
    };
    if !is_enveloped {
        return Err(malformed("type is not EnvelopedVerifiableCredential"));
    }
    match object.get("@context") {
        None => return Err(malformed("@context is absent")),
        Some(Value::Array(items)) if items.is_empty() => {
            return Err(malformed("@context is an empty array"));
        }
        Some(Value::Array(items))
            if !items
                .iter()
                .all(|item| matches!(item, Value::String(_) | Value::Object(_))) =>
        {
            return Err(malformed(
                "@context has an item that is neither a string nor an object",
            ));
        }
        Some(Value::String(_) | Value::Object(_) | Value::Array(_)) => {}
        Some(_) => {
            return Err(malformed(
                "@context is neither a string, an object nor an array",
            ));
        }
    }
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| malformed("id is not a string"))?;
    let url = split_data_url(id).ok_or_else(|| malformed("id is not a data: URL"))?;
    let media_type = url.media_type;

    let data = match media_type.as_str() {
        VC_JWT | VC_SD_JWT | VC_COSE => Some(EnvelopeData {
            data: url.data.to_owned(),
            base64: url.base64,
        }),
        _ => None,
    };
    let kid = data.as_ref().and_then(|data| kid_hint(&media_type, data));
    Ok(UnverifiedEnvelope::new(index, media_type, kid, data))
}

/// The `kid` the protected header of a `vc+jwt`, or of a `vc+sd-jwt`'s Issuer-signed JWT,
/// names, read as a hint for choosing a key and from nothing that has been verified. `None`
/// when the data cannot be read as one, which is for [`UnverifiedEnvelope::verify`] to
/// refuse, and always for a `vc+cose`, whose header is read only once its signature has
/// verified.
fn kid_hint(media_type: &str, data: &EnvelopeData) -> Option<String> {
    let text = decode_text(&data.data, data.base64).ok()?;
    match media_type {
        VC_JWT => extract_kid_from_jws(&text),
        VC_SD_JWT => extract_kid_from_jws(text.split('~').next().unwrap_or_default()),
        _ => None,
    }
}
