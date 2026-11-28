//! Each refusal from the compact JWS verifier has a distinct error.

use base64::Engine;
use serde_json::{Value, json};

use super::super::algorithm::KeyAlgorithm;
use super::super::error::{JwsError, SignatureLengthError};
use super::super::header_refusal::HeaderRefusal;
use super::super::public_key::PublicKey;
use super::super::verifier::verify_jws;
use crate::keystore::SigningMaterial;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

fn key() -> PublicKey {
    SigningMaterial::generate(KeyAlgorithm::Ed25519).public_key()
}

fn token(header: &Value, signature: &[u8]) -> String {
    format!(
        "{}.{}.{}",
        B64.encode(serde_json::to_vec(header).expect("serialises")),
        B64.encode(b"{}"),
        B64.encode(signature),
    )
}

#[test]
fn malformed_compact_serialisations_are_errors() {
    let key = key();
    for jws in ["", "a.b", "a.b.c.d"] {
        assert_eq!(
            verify_jws(jws, &key),
            Err(JwsError::MalformedCompact),
            "{jws:?}"
        );
    }
}

#[test]
fn a_signature_that_is_not_base64url_is_an_error() {
    let key = key();
    let header = B64.encode(br#"{"alg":"EdDSA"}"#);
    let jws = format!("{header}.e30.%%%");
    assert_eq!(verify_jws(&jws, &key), Err(JwsError::BadBase64));
}

#[test]
fn a_wrong_length_signature_is_an_error() {
    let key = key();
    let jws = token(&json!({"alg": "EdDSA"}), &[0; 63]);
    assert_eq!(
        verify_jws(&jws, &key),
        Err(JwsError::SignatureLength(SignatureLengthError {
            algorithm: KeyAlgorithm::Ed25519,
            expected: 64,
            actual: 63,
        }))
    );
}

#[test]
fn refused_headers_carry_the_header_refusal() {
    let key = key();
    let cases = [
        (B64.encode(b"[]"), HeaderRefusal::Unreadable),
        (
            B64.encode(br#"{"alg":"EdDSA","crit":["x"]}"#),
            HeaderRefusal::Critical,
        ),
        (
            B64.encode(br#"{"alg":"ES256"}"#),
            HeaderRefusal::Algorithm {
                found: Some("ES256".to_owned()),
            },
        ),
    ];
    for (header, refusal) in cases {
        let jws = format!("{header}.e30.{}", B64.encode([0; 64]));
        assert_eq!(
            verify_jws(&jws, &key),
            Err(JwsError::HeaderRefused(refusal)),
            "{header}"
        );
    }
}

#[test]
fn a_signature_that_does_not_verify_is_an_error() {
    let key = key();
    let jws = token(&json!({"alg": "EdDSA"}), &[0; 64]);
    assert_eq!(
        verify_jws(&jws, &key),
        Err(JwsError::SignatureDoesNotVerify)
    );
}
