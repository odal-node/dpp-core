//! The shape of an Enveloped Verifiable Credential in a presentation, checked
//! before anything is verified.
//!
//! VC-JOSE-COSE §3.1.2 says the credentials of a presentation MUST use the
//! Enveloped Verifiable Credential type of the VC Data Model, whose section on
//! Enveloped Verifiable Credentials says the object's `@context` MUST be present,
//! its `id` MUST be a `data:` URL, and its `type` MUST be
//! `EnvelopedVerifiableCredential`. A `vc+jwt` envelope's data must in addition be
//! a compact JWS (§3.1.2: the credentials are "secured using JWS").

use base64::Engine as _;
use serde_json::Value;

use dpp_crypto::jws::verifier::extract_kid_from_jws;

use super::error::JoseError;

/// The media type of a JWT-secured credential in a `data:` URL.
const VC_JWT: &str = "application/vc+jwt";

/// An envelope whose shape is right.
pub(crate) struct Envelope {
    /// The media type its `id` names, lower-cased and without parameters.
    pub(crate) media_type: String,
    /// The compact JWS inside it, for a `vc+jwt`.
    pub(crate) token: Option<String>,
    /// The `kid` that token's protected header names.
    pub(crate) kid: Option<String>,
}

/// Check the envelope at `index`, and read out what a decision needs.
///
/// Only a `vc+jwt` envelope's data is read. For any other media type the `id` is
/// held to being a `data:` URL and nothing more, since this crate cannot read the
/// format.
pub(crate) fn inspect(index: usize, value: &Value) -> Result<Envelope, JoseError> {
    let malformed = |reason: &str| JoseError::MalformedEnvelope {
        index,
        reason: reason.to_owned(),
    };
    let Some(object) = value.as_object() else {
        return Err(malformed("not a JSON object"));
    };
    if object.get("type").and_then(Value::as_str) != Some("EnvelopedVerifiableCredential") {
        return Err(malformed("type is not EnvelopedVerifiableCredential"));
    }
    if !matches!(
        object.get("@context"),
        Some(Value::String(_) | Value::Array(_))
    ) {
        return Err(malformed("@context is absent"));
    }
    let id = object
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| malformed("id is not a string"))?;
    let url = split_data_url(id).ok_or_else(|| malformed("id is not a data: URL"))?;
    let media_type = url.media_type;

    if media_type != VC_JWT {
        return Ok(Envelope {
            media_type,
            token: None,
            kid: None,
        });
    }
    let token = token_of(url.data, url.base64)
        .ok_or_else(|| malformed("the vc+jwt data is not a compact JWS"))?;
    let kid = extract_kid_from_jws(&token);
    Ok(Envelope {
        media_type,
        token: Some(token),
        kid,
    })
}

/// The parts of a `data:` URL that an envelope is read by.
struct DataUrl<'a> {
    /// The media type, lower-cased and without its parameters.
    media_type: String,
    /// Whether the parameters end in `base64`.
    base64: bool,
    /// What follows the comma, still encoded.
    data: &'a str,
}

/// Split a `data:` URL, `data:[<mediatype>][;base64],<data>`. The scheme is not
/// case-sensitive, and an empty media type means `text/plain`. Only the part
/// before the comma is read.
fn split_data_url(id: &str) -> Option<DataUrl<'_>> {
    let rest = id
        .get(..5)
        .filter(|scheme| scheme.eq_ignore_ascii_case("data:"))
        .map(|_| &id[5..])?;
    let (meta, data) = rest.split_once(',')?;
    let mut parameters = meta.split(';');
    let media_type = parameters.next().unwrap_or_default();
    let media_type = if media_type.is_empty() {
        "text/plain".to_owned()
    } else {
        media_type.to_ascii_lowercase()
    };
    Some(DataUrl {
        media_type,
        base64: parameters.next_back() == Some("base64"),
        data,
    })
}

/// The compact JWS in the data of a `vc+jwt` `data:` URL: three non-empty
/// base64url segments. The data is percent-decoded, as a `data:` URL's data is, and
/// base64-decoded first when the URL says `;base64`. The registered media type is
/// written with the JWS as the data itself, which is the form the suite uses.
fn token_of(data: &str, base64: bool) -> Option<String> {
    let mut bytes = percent_decode(data)?;
    if base64 {
        bytes = base64::engine::general_purpose::STANDARD
            .decode(bytes)
            .ok()?;
    }
    let token = String::from_utf8(bytes).ok()?;
    let segments: Vec<&str> = token.split('.').collect();
    let all_base64url = segments.len() == 3
        && segments.iter().all(|segment| {
            !segment.is_empty()
                && base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(segment)
                    .is_ok()
        });
    all_base64url.then_some(token)
}

/// `data` with its `%XX` escapes decoded, or `None` when an escape is not two hex
/// digits.
fn percent_decode(data: &str) -> Option<Vec<u8>> {
    let bytes = data.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let hex = data.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    Some(out)
}
