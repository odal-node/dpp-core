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

use super::super::verifier::{extract_kid_from_jws, resolve_verification_key, verify_jws};

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
fn fixed_key() -> super::super::PublicKey {
    let raw = hex::decode(super::common::public_hex("TEST 1")).expect("hex");
    super::super::PublicKey::from_bytes(super::super::KeyAlgorithm::Ed25519, &raw)
        .expect("an RFC 8032 key")
}

/// One published public key of each algorithm, so verification reaches the signature
/// check under all four: RFC 8032 clause 7.1's TEST 1 for Ed25519; the public key of
/// RFC 7515 Appendix A.3's P-256 example for ES256; the public point of RFC 6979
/// clause A.2.6 for ES384; and the public key of RFC 7515 Appendix A.4 for ES512.
fn fixed_keys() -> [super::super::PublicKey; 4] {
    [
        fixed_key(),
        super::common::rfc7515_key(),
        super::common::rfc6979_p384_key(),
        super::common::rfc7515_p521_key(),
    ]
}

/// Each of the W3C VC-JOSE-COSE suite's four key files, as its `secretKeyJwk` and the
/// public key its `publicKeyJwk` reads. The files are read as `keystore::tests::private_jwk::keys`
/// reads them.
fn suite_keys() -> [(serde_json::Value, super::super::PublicKey); 4] {
    [
        include_str!("../fixtures/w3c-vc-jose-cose/vm-p256.json"),
        include_str!("../fixtures/w3c-vc-jose-cose/vm-p384.json"),
        include_str!("../fixtures/w3c-vc-jose-cose/vm-p521.json"),
        include_str!("../fixtures/w3c-vc-jose-cose/vm-ed25519.json"),
    ]
    .map(|text| {
        let method: serde_json::Value = serde_json::from_str(text).expect("a verification method");
        let public = super::super::PublicKey::from_jwk(&method["publicKeyJwk"])
            .expect("the suite's public key");
        (method["secretKeyJwk"].clone(), public)
    })
}

/// The index of a suite key file, with a `d` to give it: the file's own `d`, arbitrary
/// base64url of any length, random octets of the file's field size, encoded, or text
/// that is not base64url.
fn suite_index_and_d() -> impl Strategy<Value = (usize, String)> {
    (0usize..4).prop_flat_map(|index| {
        let suite = suite_keys();
        let own = suite[index].0["d"].as_str().expect("a d").to_owned();
        let field_len = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(&own)
            .expect("base64url")
            .len();
        prop_oneof![
            Just(own),
            "[A-Za-z0-9_-]{0,200}",
            prop::collection::vec(any::<u8>(), field_len).prop_map(|octets| {
                base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(octets)
            }),
            ".{0,64}",
        ]
        .prop_map(move |d| (index, d))
    })
}

/// The key a base64url string names, under any algorithm, when it names one.
fn key_named_by(public_key_b64: &str) -> Option<super::super::PublicKey> {
    let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(public_key_b64)
        .ok()?;
    [
        super::super::KeyAlgorithm::Ed25519,
        super::super::KeyAlgorithm::EcdsaP256,
        super::super::KeyAlgorithm::EcdsaP384,
        super::super::KeyAlgorithm::EcdsaP521,
    ]
    .into_iter()
    .find_map(|algorithm| super::super::PublicKey::from_bytes(algorithm, &raw).ok())
}

proptest! {
    /// `verify_jws` must never panic on arbitrary compact-JWS-shaped input —
    /// it should return a typed refusal (`Err`),
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
        for algorithm in [
            super::super::KeyAlgorithm::Ed25519,
            super::super::KeyAlgorithm::EcdsaP256,
            super::super::KeyAlgorithm::EcdsaP384,
            super::super::KeyAlgorithm::EcdsaP521,
        ] {
            if let Ok(key) = super::super::PublicKey::from_bytes(algorithm, &bytes) {
                let _ = key.verify(b"message", &sig);
            }
        }
    }

    /// JWK parsing never panics on an arbitrary JSON value.
    #[test]
    fn public_key_from_jwk_never_panics(jwk in arb_json()) {
        let _ = super::super::PublicKey::from_jwk(&jwk);
    }

    /// A P-256 JWK whose `x` or `y` is 31 or 33 bytes is no key, whatever else it says,
    /// and parsing it never panics. Coordinates are 32 bytes or the key is refused.
    #[test]
    fn a_p256_jwk_with_a_coordinate_of_the_wrong_length_is_refused(
        x in prop::collection::vec(any::<u8>(), 31..=33),
        y in prop::collection::vec(any::<u8>(), 31..=33),
    ) {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let jwk = serde_json::json!({
            "kty": "EC",
            "crv": "P-256",
            "x": b64.encode(&x),
            "y": b64.encode(&y),
        });
        let key = super::super::PublicKey::from_jwk(&jwk);
        if x.len() != 32 || y.len() != 32 {
            prop_assert!(key.is_none());
        }
    }

    /// An ES256 JWS whose signature is 63, 64 or 65 bytes, under the RFC 7515 A.3 key,
    /// never panics, and a signature that is not 64 bytes is refused.
    #[test]
    fn es256_signatures_of_63_to_65_bytes_never_panic(
        sig in prop::collection::vec(any::<u8>(), 63..=65),
    ) {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let jws = format!(
            "{}.{}.{}",
            b64.encode(br#"{"alg":"ES256"}"#),
            b64.encode(b"{}"),
            b64.encode(&sig),
        );
        let verdict = verify_jws(&jws, &super::common::rfc7515_key());
        if sig.len() != 64 {
            prop_assert!(verdict.is_err());
        }
    }

    /// A P-384 JWK whose `x` or `y` is 47 or 49 bytes is no key, whatever else it says,
    /// and parsing it never panics. Coordinates are 48 bytes or the key is refused.
    #[test]
    fn a_p384_jwk_with_a_coordinate_of_the_wrong_length_is_refused(
        x in prop::collection::vec(any::<u8>(), 47..=49),
        y in prop::collection::vec(any::<u8>(), 47..=49),
    ) {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let jwk = serde_json::json!({
            "kty": "EC",
            "crv": "P-384",
            "x": b64.encode(&x),
            "y": b64.encode(&y),
        });
        let key = super::super::PublicKey::from_jwk(&jwk);
        if x.len() != 48 || y.len() != 48 {
            prop_assert!(key.is_none());
        }
    }

    /// A P-521 JWK whose `x` or `y` is 65 or 67 bytes is no key, whatever else it says,
    /// and parsing it never panics. Coordinates are 66 bytes or the key is refused.
    #[test]
    fn a_p521_jwk_with_a_coordinate_of_the_wrong_length_is_refused(
        x in prop::collection::vec(any::<u8>(), 65..=67),
        y in prop::collection::vec(any::<u8>(), 65..=67),
    ) {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let jwk = serde_json::json!({
            "kty": "EC",
            "crv": "P-521",
            "x": b64.encode(&x),
            "y": b64.encode(&y),
        });
        let key = super::super::PublicKey::from_jwk(&jwk);
        if x.len() != 66 || y.len() != 66 {
            prop_assert!(key.is_none());
        }
    }

    /// An ES384 JWS whose signature is 95, 96 or 97 bytes, under a P-384 key, never
    /// panics, and a signature that is not 96 bytes is refused.
    #[test]
    fn es384_signatures_of_95_to_97_bytes_never_panic(
        sig in prop::collection::vec(any::<u8>(), 95..=97),
    ) {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let key = crate::keystore::SigningMaterial::generate(super::super::KeyAlgorithm::EcdsaP384)
            .public_key();
        let jws = format!(
            "{}.{}.{}",
            b64.encode(br#"{"alg":"ES384"}"#),
            b64.encode(b"{}"),
            b64.encode(&sig),
        );
        let verdict = verify_jws(&jws, &key);
        if sig.len() != 96 {
            prop_assert!(verdict.is_err());
        }
    }

    /// An ES512 JWS whose signature is 131, 132 or 133 bytes, under a P-521 key, never
    /// panics, and a signature that is not 132 bytes is refused.
    #[test]
    fn es512_signatures_of_131_to_133_bytes_never_panic(
        sig in prop::collection::vec(any::<u8>(), 131..=133),
    ) {
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        let key = crate::keystore::SigningMaterial::generate(super::super::KeyAlgorithm::EcdsaP521)
            .public_key();
        let jws = format!(
            "{}.{}.{}",
            b64.encode(br#"{"alg":"ES512"}"#),
            b64.encode(b"{}"),
            b64.encode(&sig),
        );
        let verdict = verify_jws(&jws, &key);
        if sig.len() != 132 {
            prop_assert!(verdict.is_err());
        }
    }

    /// Private-JWK parsing never panics on a suite key with its `d` replaced, and an `Ok`
    /// entry holds the public key the file names. Each case starts from one of the suite's
    /// four `secretKeyJwk` values, so the public members and the rules after them are
    /// reached, and varies `d` alone: the file's own `d`, which builds, arbitrary base64url
    /// of any length, random octets of the file's field size, or text that is not
    /// base64url.
    #[test]
    fn private_jwk_never_panics((index, d) in suite_index_and_d()) {
        let suite = suite_keys();
        let (secret, public) = &suite[index];
        let mut jwk = secret.clone();
        jwk["d"] = serde_json::json!(d);
        if let Ok(entry) = crate::keystore::KeyEntry::from_private_jwk(&jwk) {
            prop_assert_eq!(entry.public_key(), public.clone());
        }
    }

    /// Private-JWK parsing never panics on an arbitrary JSON value.
    #[test]
    fn private_jwk_from_arbitrary_json_never_panics(jwk in arb_json()) {
        let _ = crate::keystore::KeyEntry::from_private_jwk(&jwk);
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
