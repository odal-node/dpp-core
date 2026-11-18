//! The media-type typing of a JWS protected header, shared by the two JWT-secured
//! credential paths: the DPP access credential and the generic VC-JOSE-COSE
//! credential and presentation.

use base64::Engine as _;

/// Whether a media-type header value names `expected`. RFC 7515 §4.1.9 lets a
/// producer omit the `application/` prefix and has a recipient read the value as
/// if it were there, and media types compare case-insensitively.
pub(crate) fn names_media_type(value: &str, expected: &str) -> bool {
    let value = value.to_ascii_lowercase();
    value.strip_prefix("application/").unwrap_or(&value) == expected
}

/// Why a protected header does not type the token as the one asked for.
///
/// A `found` is the member as JSON text (a string keeps its quotes), or `None`
/// when the member is absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TypeRefusal {
    /// The first segment is not a base64url JSON document.
    Unreadable,
    /// `typ` is absent, or is not the media type asked for.
    Typ { found: Option<String> },
    /// `cty` is present and is not the content type asked for.
    Cty { found: String },
}

/// The refusal, if the protected header does not type the token as `typ`: `typ`
/// must be present and name `typ`, and a `cty`, if present, must name `cty`.
///
/// RFC 7515 §4.1.9 says `typ` is for the application's use, and RFC 8725 §3.11
/// recommends explicit typing so that a token of one kind is not taken for another.
pub(crate) fn header_type_refusal(jws: &str, typ: &str, cty: &str) -> Option<TypeRefusal> {
    let Some(header) = jws
        .split('.')
        .next()
        .and_then(|h| {
            base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(h)
                .ok()
        })
        .and_then(|bytes| serde_json::from_slice::<serde_json::Value>(&bytes).ok())
    else {
        return Some(TypeRefusal::Unreadable);
    };
    let found = header.get("typ");
    if !found
        .and_then(serde_json::Value::as_str)
        .is_some_and(|t| names_media_type(t, typ))
    {
        return Some(TypeRefusal::Typ {
            found: found.map(ToString::to_string),
        });
    }
    match header.get("cty") {
        None => None,
        Some(found) if found.as_str().is_some_and(|c| names_media_type(c, cty)) => None,
        Some(found) => Some(TypeRefusal::Cty {
            found: found.to_string(),
        }),
    }
}
