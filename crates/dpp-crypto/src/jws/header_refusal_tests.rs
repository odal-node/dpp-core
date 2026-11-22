//! [`header_refusal`]: each reason a protected header is refused, and the cases it is
//! not.
//!
//! A header is built by hand and wrapped in a token whose other segments do not
//! matter, because only the first segment is read. Each refusal is paired with the
//! control that is not refused, so a refusal cannot be a flaw in the construction.
//! The last test ties the function to [`verify_jws`]: every token this function
//! refuses is a token `verify_jws` answers `false` for, even with a genuine
//! signature, so splitting the one check into a typed one changed no verdict.

use base64::Engine;
use serde_json::{Value, json};

use super::algorithm::KeyAlgorithm;
use super::header_refusal::HeaderRefusal;
use super::verifier::{header_refusal, verify_jws};
use crate::test_support::temp_store;

const B64: base64::engine::general_purpose::GeneralPurpose =
    base64::engine::general_purpose::URL_SAFE_NO_PAD;

/// A compact JWS whose protected header is `header` and whose payload and signature
/// are placeholders.
fn token(header: &Value) -> String {
    format!(
        "{}.{}.{}",
        B64.encode(serde_json::to_vec(header).expect("serialises")),
        B64.encode(b"{}"),
        B64.encode([0u8; 64]),
    )
}

#[test]
fn a_header_naming_the_keys_algorithm_is_not_refused() {
    let cases = [
        (KeyAlgorithm::Ed25519, "EdDSA"),
        (KeyAlgorithm::Ed25519, "Ed25519"),
        (KeyAlgorithm::EcdsaP256, "ES256"),
        (KeyAlgorithm::EcdsaP384, "ES384"),
        (KeyAlgorithm::EcdsaP521, "ES512"),
    ];
    for (algorithm, alg) in cases {
        assert_eq!(
            header_refusal(&token(&json!({"alg": alg})), algorithm),
            None,
            "{alg}"
        );
    }
}

#[test]
fn an_unknown_member_is_not_a_refusal() {
    let header = json!({"alg": "ES256", "kid": "k", "typ": "vc+jwt", "x": 1, "b64": false});
    assert_eq!(
        header_refusal(&token(&header), KeyAlgorithm::EcdsaP256),
        None
    );
}

#[test]
fn an_unreadable_header_is_unreadable() {
    let not_an_object = [
        B64.encode(b"[\"alg\"]"),
        B64.encode(b"\"ES256\""),
        B64.encode(b"null"),
        B64.encode(b"not json"),
        B64.encode(b""),
    ];
    for segment in not_an_object {
        let jws = format!("{segment}.e30.AA");
        assert_eq!(
            header_refusal(&jws, KeyAlgorithm::EcdsaP256),
            Some(HeaderRefusal::Unreadable),
            "{segment}"
        );
    }
    for jws in ["", ".", "!!!.e30.AA", "{\"alg\":\"ES256\"}.e30.AA"] {
        assert_eq!(
            header_refusal(jws, KeyAlgorithm::EcdsaP256),
            Some(HeaderRefusal::Unreadable),
            "{jws:?}"
        );
    }
}

#[test]
fn every_shape_of_crit_is_critical() {
    for crit in [
        json!([]),
        json!(["b64"]),
        json!(["alg"]),
        json!("b64"),
        json!({}),
        json!(null),
    ] {
        let header = json!({"alg": "ES256", "crit": crit, "b64": false});
        assert_eq!(
            header_refusal(&token(&header), KeyAlgorithm::EcdsaP256),
            Some(HeaderRefusal::Critical),
            "crit: {crit}"
        );
    }
}

/// `crit` is read before `alg`, so a token that is wrong in both ways is reported as
/// the `crit` it carries.
#[test]
fn crit_is_reported_before_a_wrong_algorithm() {
    let header = json!({"alg": "none", "crit": ["b64"]});
    assert_eq!(
        header_refusal(&token(&header), KeyAlgorithm::EcdsaP256),
        Some(HeaderRefusal::Critical)
    );
}

#[test]
fn another_algorithm_is_refused_with_what_the_header_said() {
    let header = json!({"alg": "ES384"});
    assert_eq!(
        header_refusal(&token(&header), KeyAlgorithm::EcdsaP256),
        Some(HeaderRefusal::Algorithm {
            found: Some("ES384".to_owned())
        })
    );
    let none = json!({"alg": "none"});
    assert_eq!(
        header_refusal(&token(&none), KeyAlgorithm::Ed25519),
        Some(HeaderRefusal::Algorithm {
            found: Some("none".to_owned())
        })
    );
}

#[test]
fn an_absent_or_non_string_algorithm_is_refused_with_nothing_found() {
    for header in [
        json!({}),
        json!({"kid": "k"}),
        json!({"alg": 256}),
        json!({"alg": null}),
    ] {
        assert_eq!(
            header_refusal(&token(&header), KeyAlgorithm::EcdsaP256),
            Some(HeaderRefusal::Algorithm { found: None }),
            "{header}"
        );
    }
}

/// A genuine signature over a refused header is still refused by `verify_jws`, so the
/// typed reason and the boolean agree.
#[test]
fn verify_jws_refuses_exactly_what_header_refusal_refuses() {
    let store = temp_store("header-refusal", "k");
    let key = store.load_key("k").expect("load key");
    let public = key.public_key();
    let sign = |header: &Value| {
        let input = format!(
            "{}.{}",
            B64.encode(serde_json::to_vec(header).expect("serialises")),
            B64.encode(b"{}")
        );
        format!(
            "{input}.{}",
            B64.encode(key.sign(input.as_bytes()).expect("an unrevoked key signs"))
        )
    };

    for (header, refused) in [
        (json!({"alg": "EdDSA"}), false),
        (json!({"alg": "EdDSA", "crit": ["x"], "x": 1}), true),
        (json!({"alg": "ES256"}), true),
        (json!({"alg": "none"}), true),
        (json!({}), true),
    ] {
        let jws = sign(&header);
        assert_eq!(
            header_refusal(&jws, public.algorithm()).is_some(),
            refused,
            "{header}"
        );
        assert_eq!(
            verify_jws(&jws, &public).expect("well-formed"),
            !refused,
            "{header}"
        );
    }
}
