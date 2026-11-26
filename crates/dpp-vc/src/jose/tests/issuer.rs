//! Verifying a credential's issuer: `iss` against `issuer` (VC-JOSE-COSE §4.1.2), and
//! the absolute `kid` when there is no `iss` (§4.2).
//!
//! Tokens are forged, as in `verify.rs`: a genuine signature over a header or
//! payload `sign_credential` would not write, so a refusal can only be the check it
//! names.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;

use super::super::VerifyOptions;
use super::super::{JoseError, verify_credential};
use crate::test_support::{credential, forge, generated, with_credential, with_header};

fn key() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP256)
}

fn verify(token: &str, key: &KeyEntry) -> Result<Value, JoseError> {
    verify_credential(token, &key.public_key(), VerifyOptions::new())
}

// ── §4.1.2: `iss` is the issuer ───────────────────────────────────────────────

#[test]
fn iss_equal_to_a_string_issuer_verifies() {
    let key = key();
    let token = with_credential(&key, |c| c["iss"] = json!("https://issuer.example"));

    assert!(verify(&token, &key).is_ok());
}

#[test]
fn iss_that_is_not_a_string_issuer_is_refused() {
    let key = key();
    let token = with_credential(&key, |c| c["iss"] = json!("https://attacker.example"));

    assert_eq!(
        verify(&token, &key),
        Err(JoseError::IssuerMismatch {
            iss: "https://attacker.example".to_owned(),
            issuer: Some("https://issuer.example".to_owned()),
        })
    );
}

#[test]
fn iss_is_held_to_the_id_of_an_object_issuer() {
    let key = key();
    let object = json!({"id": "https://issuer.example", "name": "Issuer"});
    let matching = with_credential(&key, |c| {
        c["issuer"] = object.clone();
        c["iss"] = json!("https://issuer.example");
    });
    let mismatch = with_credential(&key, |c| {
        c["issuer"] = object.clone();
        c["iss"] = json!("https://attacker.example");
    });

    assert!(verify(&matching, &key).is_ok());
    assert_eq!(
        verify(&mismatch, &key),
        Err(JoseError::IssuerMismatch {
            iss: "https://attacker.example".to_owned(),
            issuer: Some("https://issuer.example".to_owned()),
        })
    );
}

/// An `iss` with no issuer to match is a mismatch, not a pass: a credential that
/// names no issuer cannot be vouched for by a claim that names one.
#[test]
fn iss_without_a_readable_issuer_is_refused() {
    let key = key();
    for issuer in [json!({"name": "no id"}), json!(7), json!(null)] {
        let token = with_credential(&key, |c| {
            c["issuer"] = issuer.clone();
            c["iss"] = json!("https://issuer.example");
        });
        assert_eq!(
            verify(&token, &key),
            Err(JoseError::IssuerMismatch {
                iss: "https://issuer.example".to_owned(),
                issuer: None,
            })
        );
    }
    let absent = with_credential(&key, |c| {
        c.as_object_mut().expect("an object").remove("issuer");
        c["iss"] = json!("https://issuer.example");
    });
    assert!(matches!(
        verify(&absent, &key),
        Err(JoseError::IssuerMismatch { issuer: None, .. })
    ));
}

#[test]
fn an_iss_that_is_not_a_string_is_refused() {
    let key = key();
    let token = with_credential(&key, |c| c["iss"] = json!(["https://issuer.example"]));

    assert!(matches!(
        verify(&token, &key),
        Err(JoseError::MalformedPayload { .. })
    ));
}

// ── §4.2: the `kid` when there is no `iss` ────────────────────────────────────

/// §4.2 opens "When using controlled identifier documents with this specification", and a
/// verifier here is given its key, so it holds the `kid` against no document. The suite's own
/// COSE rows 29 and 31 are bare thumbprints with no `iss`, and expect success.
#[test]
fn without_an_iss_a_bare_or_absent_kid_verifies_for_a_url_issuer() {
    let key = key();
    let bare = with_header(&key, |h| {
        h["kid"] = json!("73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU")
    });
    let absent = with_header(&key, |h| {
        h.as_object_mut().expect("an object").remove("kid");
    });

    assert!(verify(&bare, &key).is_ok());
    assert!(verify(&absent, &key).is_ok());
    assert!(verify(&with_header(&key, |_| {}), &key).is_ok());
}

#[test]
fn an_iss_passes_and_a_non_url_issuer_is_refused_as_an_invalid_issuer() {
    let key = key();
    let with_iss = forge(&key, &json!({"alg": "ES256", "typ": "vc+jwt"}), &{
        let mut c = credential();
        c["iss"] = json!("https://issuer.example");
        c
    });
    let not_a_url = forge(
        &key,
        &json!({"alg": "ES256", "typ": "vc+jwt", "kid": "bare"}),
        &{
            let mut c = credential();
            c["issuer"] = json!("Example Issuer Ltd");
            c
        },
    );

    assert!(verify(&with_iss, &key).is_ok());
    // A non-URL issuer is refused for the issuer (VC DM 2.0 §4.7), not for the bare kid.
    assert!(matches!(
        verify(&not_a_url, &key),
        Err(JoseError::InvalidIssuer { .. })
    ));
}
