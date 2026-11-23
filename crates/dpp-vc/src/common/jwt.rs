//! Reading the parts of a compact JWS, shared by the credential paths that look
//! inside a token they have already verified.

use base64::Engine;
use serde_json::{Map, Value};

/// Part `index` of a compact JWS, the protected header (0) or the payload (1), as
/// a JSON object.
///
/// Reads, never verifies: the caller has checked the signature, so what this
/// returns is what the signer wrote.
pub(crate) fn jwt_part(jwt: &str, index: usize) -> Option<Map<String, Value>> {
    let part = jwt.split('.').nth(index)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(part)
        .ok()?;
    match serde_json::from_slice(&bytes).ok()? {
        Value::Object(map) => Some(map),
        _ => None,
    }
}
