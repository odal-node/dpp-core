//! Property tests for the JWS verifier's hostile-input surface.
//!
//! `verify_jws` parses an attacker-controlled compact JWS (which can originate
//! from a scanned QR code or URL), and `resolve_verification_key` parses one
//! against a DID document fetched over the network —
//! neither input is trusted. The sibling `dpp-digital-link` crate already
//! covers this class of risk for `DigitalLink::parse` with a proptest harness;
//! this file gives the JWS verifier the same treatment.

use proptest::prelude::*;

use base64::Engine;

use super::verifier::{extract_kid_from_jws, resolve_verification_key, verify_jws};

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

    /// `resolve_verification_key` must never panic on an arbitrary (malformed,
    /// wrong-shaped, or adversarial) JSON value standing in for a fetched DID
    /// document, whatever the JWS is.
    #[test]
    fn resolve_verification_key_never_panics(doc in arb_json(), jws in ".{0,128}") {
        let _ = resolve_verification_key(&doc, &jws);
    }

    /// The same, with a JWS whose header decodes, so the pressure lands on the
    /// search through the document and not on the first parse.
    #[test]
    fn resolve_verification_key_never_panics_with_a_readable_header(
        doc in arb_json(), kid in ".{0,64}", alg in ".{0,16}"
    ) {
        let header = serde_json::json!({"kid": kid, "alg": alg}).to_string();
        let jws = format!(
            "{}.e30.c2ln",
            base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(header)
        );
        let _ = resolve_verification_key(&doc, &jws);
    }
}
