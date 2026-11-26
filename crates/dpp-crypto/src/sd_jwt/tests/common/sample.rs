//! A stub token and a sample claim set the disclosure tests build on.

use serde_json::{Map, Value, json};

/// Sign nothing — these tests exercise the disclosure mechanism, and the JWS
/// check is [`crate::jws::verifier`]'s. A stub JWT with a real payload is all
/// the mechanism needs, and using one keeps a key out of tests that do not
/// depend on signing.
pub(crate) fn stub_jwt(payload: &Value) -> String {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    format!(
        "{}.{}.{}",
        b64.encode(br#"{"alg":"EdDSA","typ":"dc+sd-jwt"}"#),
        b64.encode(payload.to_string()),
        b64.encode([0u8; 64])
    )
}

pub(crate) fn sample() -> Map<String, Value> {
    json!({
        "batteryChemistry": "LFP",
        "cathodeMaterial": ["LiFePO4"],
        "stateOfHealthPct": 87.5,
        "testReportResults": "report-4471",
        "safetyMeasures": ["Do not puncture"],
    })
    .as_object()
    .unwrap()
    .clone()
}

pub(crate) const HIDDEN: [&str; 4] = [
    "cathodeMaterial",
    "stateOfHealthPct",
    "testReportResults",
    "safetyMeasures",
];
