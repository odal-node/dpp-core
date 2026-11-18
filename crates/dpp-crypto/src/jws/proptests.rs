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

/// The public key of RFC 8032 clause 7.1's TEST 1, so verification reaches the
/// signature check.
fn fixed_key() -> super::PublicKey {
    let raw = hex::decode(super::rfc8032_vector_tests::public_hex("TEST 1")).expect("hex");
    super::PublicKey::from_bytes(super::KeyAlgorithm::Ed25519, &raw).expect("an RFC 8032 key")
}

/// One published public key of each algorithm, so verification reaches the signature
/// check under both: RFC 8032 clause 7.1's TEST 1 for Ed25519, and the public key of
/// RFC 7515 Appendix A.3's P-256 example for ES256.
fn fixed_keys() -> [super::PublicKey; 2] {
    [fixed_key(), super::es256_tests::rfc7515_key()]
}

/// The key a base64url string names, under either algorithm, when it names one.
fn key_named_by(public_key_b64: &str) -> Option<super::PublicKey> {
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(public_key_b64)
        .ok()?;
    [super::KeyAlgorithm::Ed25519, super::KeyAlgorithm::EcdsaP256]
        .into_iter()
        .find_map(|algorithm| super::PublicKey::from_bytes(algorithm, &raw).ok())
}

proptest! {
    /// `verify_jws` must never panic on arbitrary compact-JWS-shaped input —
    /// it should fail closed (`Ok(false)`) or report a parse error (`Err`),
    /// never crash the process that's verifying an untrusted signature. The key
    /// is fuzzed as well: a string that names a key is verified under that key,
    /// and any other string under each of `fixed_keys()`.
    #[test]
    fn verify_jws_never_panics(jws in ".{0,128}", public_key_b64 in ".{0,64}") {
        match key_named_by(&public_key_b64) {
            Some(key) => {
                let _ = verify_jws(&jws, &key);
            }
            None => {
                for key in fixed_keys() {
                    let _ = verify_jws(&jws, &key);
                }
            }
        }
    }

    /// Same property, restricted to strings that at least have the right
    /// number of `.`-separated parts, so the fuzzing pressure lands past the
    /// early `parts.len() != 3` return and into the base64/JSON parsing paths.
    #[test]
    fn verify_jws_never_panics_three_part_shape(
        header in ".{0,32}", payload in ".{0,64}", sig in ".{0,32}"
    ) {
        let jws = format!("{header}.{payload}.{sig}");
        for key in fixed_keys() {
            let _ = verify_jws(&jws, &key);
        }
    }

    /// Key decoding never panics on arbitrary bytes, for any algorithm, and an
    /// encoding it accepts verifies arbitrary signatures without panicking.
    #[test]
    fn public_key_decoding_never_panics(bytes in prop::collection::vec(any::<u8>(), 0..80),
                                        sig in prop::collection::vec(any::<u8>(), 0..80)) {
        for algorithm in [super::KeyAlgorithm::Ed25519, super::KeyAlgorithm::EcdsaP256] {
            if let Ok(key) = super::PublicKey::from_bytes(algorithm, &bytes) {
                let _ = key.verify(b"message", &sig);
            }
        }
    }

    /// JWK parsing never panics on an arbitrary JSON value.
    #[test]
    fn public_key_from_jwk_never_panics(jwk in arb_json()) {
        let _ = super::PublicKey::from_jwk(&jwk);
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
