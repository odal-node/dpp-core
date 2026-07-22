//! Property tests for the JWS verifier's hostile-input surface.
//!
//! `verify_jws` parses an attacker-controlled compact JWS (which can originate
//! from a scanned QR code or URL), and `extract_primary_public_key`/
//! `extract_key_by_fingerprint` parse a DID document fetched over the network —
//! neither input is trusted. The sibling `dpp-digital-link` crate already
//! covers this class of risk for `DigitalLink::parse` with a proptest harness;
//! this file gives the JWS verifier the same treatment.

use proptest::prelude::*;

use super::verifier::{
    extract_key_by_fingerprint, extract_kid_from_jws, extract_primary_public_key, verify_jws,
};

/// A bounded-depth, arbitrary JSON value — stands in for a malformed or
/// adversarial DID document.
fn arb_json() -> impl Strategy<Value = serde_json::Value> {
    let leaf = prop_oneof![
        Just(serde_json::Value::Null),
        any::<bool>().prop_map(serde_json::Value::Bool),
        any::<i64>().prop_map(|n| serde_json::json!(n)),
        ".{0,16}".prop_map(serde_json::Value::String),
    ];
    leaf.prop_recursive(3, 32, 5, |inner| {
        prop_oneof![
            prop::collection::vec(inner.clone(), 0..5).prop_map(serde_json::Value::Array),
            prop::collection::hash_map(".{0,8}", inner, 0..5)
                .prop_map(|m| serde_json::Value::Object(m.into_iter().collect())),
        ]
    })
}

proptest! {
    /// `verify_jws` must never panic on arbitrary compact-JWS-shaped input —
    /// it should fail closed (`Ok(false)`) or report a parse error (`Err`),
    /// never crash the process that's verifying an untrusted signature.
    #[test]
    fn verify_jws_never_panics(jws in ".{0,128}", public_key_b64 in ".{0,64}") {
        let _ = verify_jws(&jws, &public_key_b64);
    }

    /// Same property, restricted to strings that at least have the right
    /// number of `.`-separated parts, so the fuzzing pressure lands past the
    /// early `parts.len() != 3` return and into the base64/JSON parsing paths.
    #[test]
    fn verify_jws_never_panics_three_part_shape(
        header in ".{0,32}", payload in ".{0,64}", sig in ".{0,32}"
    ) {
        let jws = format!("{header}.{payload}.{sig}");
        let _ = verify_jws(&jws, &sig);
    }

    /// `extract_kid_from_jws` must never panic on arbitrary input.
    #[test]
    fn extract_kid_never_panics(jws in ".{0,128}") {
        let _ = extract_kid_from_jws(&jws);
    }

    /// `extract_primary_public_key`/`extract_key_by_fingerprint` must never
    /// panic on an arbitrary (malformed, wrong-shaped, or adversarial) JSON
    /// value standing in for a fetched DID document.
    #[test]
    fn extract_primary_public_key_never_panics(doc in arb_json()) {
        let _ = extract_primary_public_key(&doc);
    }

    #[test]
    fn extract_key_by_fingerprint_never_panics(doc in arb_json(), kid in ".{0,64}") {
        let _ = extract_key_by_fingerprint(&doc, &kid);
    }
}
