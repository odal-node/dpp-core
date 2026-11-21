//! The media-type typing of a JWS or `COSE_Sign1` protected header, shared by the
//! JWT-secured credential paths, the DPP access credential and the generic VC-JOSE-COSE
//! credential and presentation, and by the COSE path, which holds `typ` and the content type
//! to the same policy as the JWS `typ` and `cty`.

use base64::Engine as _;

use dpp_crypto::cose::ContentType;

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

/// The refusal, if the `typ` and content type (labels 16 and 3) of a COSE protected header do
/// not type the message as `typ`: held to the policy [`header_type_refusal`] applies to a JWS,
/// so a `typ` is required and must name `typ`, and a content type, if present, must name
/// `cty`, each with or without the `application/` prefix and without regard to case.
///
/// A value that is not a text string, a CoAP Content-Format identifier, names no media type
/// and is refused. A `found` is the value as JSON text (a string keeps its quotes) or the
/// identifier's digits, and `None` for a `typ` that is absent.
pub(crate) fn cose_type_refusal(
    typ: Option<&ContentType>,
    content_type: Option<&ContentType>,
    expected_typ: &str,
    expected_cty: &str,
) -> Option<TypeRefusal> {
    fn shown(value: &ContentType) -> String {
        match value {
            ContentType::Text(text) => serde_json::Value::String(text.clone()).to_string(),
            ContentType::CoapFormat(id) => id.to_string(),
            _ => "an unknown form".to_owned(),
        }
    }
    let names = |value: &ContentType, expected: &str| {
        value
            .as_text()
            .is_some_and(|text| names_media_type(text, expected))
    };
    if !typ.is_some_and(|typ| names(typ, expected_typ)) {
        return Some(TypeRefusal::Typ {
            found: typ.map(shown),
        });
    }
    match content_type {
        Some(found) if !names(found, expected_cty) => Some(TypeRefusal::Cty {
            found: shown(found),
        }),
        _ => None,
    }
}
