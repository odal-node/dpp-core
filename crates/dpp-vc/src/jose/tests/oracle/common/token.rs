//! Reading a compact JWS apart, without checking anything.

use base64::Engine as _;
use serde_json::Value;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// The three segments of a compact JWS.
fn segments(token: &str) -> [&str; 3] {
    let mut it = token.split('.');
    let found = [it.next(), it.next(), it.next()];
    assert!(it.next().is_none(), "a compact JWS has three segments");
    found.map(|s| s.expect("a compact JWS has three segments"))
}

/// The protected header, decoded.
pub(crate) fn header(token: &str) -> Value {
    let bytes = B64.decode(segments(token)[0]).expect("base64url");
    serde_json::from_slice(&bytes).expect("a JSON header")
}

/// The raw signature, `R ‖ S` for ECDSA.
pub(crate) fn signature(token: &str) -> Vec<u8> {
    B64.decode(segments(token)[2]).expect("base64url")
}

/// The same token with its signature replaced.
pub(crate) fn with_signature(token: &str, signature: &[u8]) -> String {
    let [header, payload, _] = segments(token);
    format!("{header}.{payload}.{}", B64.encode(signature))
}

/// The compact JWS of every `vc+jwt` envelope in a presentation's
/// `verifiableCredential`, read as text, with its position.
pub(crate) fn envelope_tokens(presentation: &Value) -> Vec<(usize, String)> {
    presentation["verifiableCredential"]
        .as_array()
        .map(|list| {
            list.iter()
                .enumerate()
                .filter_map(|(i, e)| {
                    let id = e["id"].as_str()?;
                    Some((i, id.strip_prefix("data:application/vc+jwt,")?.to_owned()))
                })
                .collect()
        })
        .unwrap_or_default()
}
