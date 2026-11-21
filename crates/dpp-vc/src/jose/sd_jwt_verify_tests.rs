//! Verifying a credential or a presentation secured as an SD-JWT: each check refuses on its
//! own, with a control that passes.
//!
//! Tokens are forged where a refusal has to be only the check it names: a genuine
//! signature over a header or payload that `sign_credential_sd_jwt` would never write. The
//! suite's inputs are in `suite_sdjwt_tests`.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;
use dpp_crypto::sd_jwt::{Disclosure, SdJwt};

use super::VerifyOptions;
use super::{JoseError, sign_credential_sd_jwt, verify_credential, verify_credential_sd_jwt};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::{
    KID, credential, forge, forge_sd, forge_sd_concealing, generated, sd_credential_header,
};

fn key() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP256)
}

fn verify(token: &str, key: &KeyEntry) -> Result<Value, JoseError> {
    verify_credential_sd_jwt(
        token,
        &key.public_key(),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    )
}

/// The control: a token with nothing wrong verifies, so every refusal below is the thing
/// changed.
#[test]
fn a_forged_credential_with_nothing_wrong_verifies() {
    let key = key();

    let token = forge_sd(&key, &sd_credential_header(&key), &credential(), &[]);

    assert_eq!(verify(&token, &key), Ok(credential()));
}

// ── the combined format ───────────────────────────────────────────────────────

#[test]
fn something_that_is_not_the_combined_format_is_refused() {
    let key = key();
    let jwt = forge(&key, &sd_credential_header(&key), &credential());
    let lost_tilde = {
        let disclosure = Disclosure::new("x", json!(1)).expect("builds");
        format!("{jwt}~{}", disclosure.encoded())
    };
    for text in [
        String::new(),
        "abc".to_owned(),
        jwt.clone(),
        lost_tilde,
        format!("{jwt}~not base64!~"),
    ] {
        assert!(
            matches!(verify(&text, &key), Err(JoseError::MalformedSdJwt { .. })),
            "{text:?}"
        );
    }
}

#[test]
fn a_token_with_no_dots_in_its_jwt_is_not_a_compact_jws() {
    assert_eq!(verify("abc~", &key()), Err(JoseError::NotCompactJws));
}

/// A `vc+jwt` is not an SD-JWT, and the other way round the SD-JWT is not a compact JWS
/// that verifies.
#[test]
fn the_two_forms_are_not_taken_for_each_other() {
    let key = key();
    let sd = sign_credential_sd_jwt(&credential(), &key, KID, &[], &[], None).expect("signs");
    let jws = super::sign_credential(&credential(), &key, KID, &[]).expect("signs");

    assert!(matches!(
        verify(&jws, &key),
        Err(JoseError::MalformedSdJwt { .. })
    ));
    assert!(verify_credential(&sd.serialise(), &key.public_key(), VerifyOptions::new()).is_err());
}

// ── the Issuer-signed JWT ─────────────────────────────────────────────────────

fn with_header(key: &KeyEntry, change: impl FnOnce(&mut Value)) -> String {
    let mut header = sd_credential_header(key);
    change(&mut header);
    forge_sd(key, &header, &credential(), &[])
}

#[test]
fn the_header_is_held_to_the_rules_of_the_vc_jwt_form() {
    let key = key();

    assert_eq!(
        verify(&with_header(&key, |h| h["crit"] = json!(["x"])), &key),
        Err(JoseError::UnsupportedCritical)
    );
    assert_eq!(
        verify(&with_header(&key, |h| h["b64"] = json!(false)), &key),
        Err(JoseError::UnencodedPayload)
    );
    for alg in ["none", "ES384", "EdDSA"] {
        assert_eq!(
            verify(&with_header(&key, |h| h["alg"] = json!(alg)), &key),
            Err(JoseError::AlgorithmMismatch {
                header_alg: Some(alg.to_owned()),
                key_alg: KeyAlgorithm::EcdsaP256,
            }),
            "{alg}"
        );
    }
}

#[test]
fn typ_is_required_and_names_vc_sd_jwt() {
    let key = key();
    let wrong = |found: Option<&str>, member: &'static str, expected: &'static str| {
        Err(JoseError::WrongMediaType {
            member,
            found: found.map(str::to_owned),
            expected,
        })
    };

    assert_eq!(
        verify(
            &with_header(&key, |h| {
                h.as_object_mut().expect("an object").remove("typ");
            }),
            &key
        ),
        wrong(None, "typ", "vc+sd-jwt")
    );
    for typ in ["vc+jwt", "dc+sd-jwt", "vp+sd-jwt", "bad+typ"] {
        assert_eq!(
            verify(&with_header(&key, |h| h["typ"] = json!(typ)), &key),
            wrong(Some(&format!("\"{typ}\"")), "typ", "vc+sd-jwt"),
            "{typ}"
        );
    }
    assert_eq!(
        verify(&with_header(&key, |h| h["cty"] = json!("bad+cty")), &key),
        wrong(Some("\"bad+cty\""), "cty", "vc")
    );
    for typ in ["application/vc+sd-jwt", "Application/VC+SD-JWT"] {
        assert!(
            verify(&with_header(&key, |h| h["typ"] = json!(typ)), &key).is_ok(),
            "{typ}"
        );
    }
    assert!(
        verify(
            &with_header(&key, |h| {
                h.as_object_mut().expect("an object").remove("cty");
            }),
            &key
        )
        .is_ok()
    );
}

#[test]
fn a_signature_that_does_not_verify_is_refused_before_the_disclosures_are_read() {
    let key = key();
    let other = generated(KeyAlgorithm::EcdsaP256);
    let unmatched = Disclosure::new("stray", json!(1)).expect("builds");
    let token = forge_sd(
        &other,
        &sd_credential_header(&other),
        &credential(),
        &[unmatched],
    );

    assert_eq!(verify(&token, &key), Err(JoseError::SignatureDoesNotVerify));
}

// ── Key Binding ───────────────────────────────────────────────────────────────

/// When Key Binding is not required, a Key Binding JWT the token carries is not read
/// (RFC 9901 §7.3 steps 3 and 4), so the token verifies as it does without it. The Issuer's
/// signature is still checked first.
#[test]
fn a_key_binding_jwt_is_ignored_when_binding_is_not_required() {
    let key = key();
    let signed = forge(&key, &sd_credential_header(&key), &credential());
    let kb = forge(
        &key,
        &json!({"alg": "ES256", "typ": "kb+jwt"}),
        &json!({"nonce": "n"}),
    );

    assert_eq!(verify(&format!("{signed}~{kb}"), &key), Ok(credential()));
    let other = generated(KeyAlgorithm::EcdsaP256);
    assert_eq!(
        verify(&format!("{signed}~{kb}"), &other),
        Err(JoseError::SignatureDoesNotVerify),
        "the signature is checked first"
    );
}

// ── the Disclosures ───────────────────────────────────────────────────────────

fn concealing(key: &KeyEntry, conceal: &[&str]) -> String {
    forge_sd_concealing(key, &sd_credential_header(key), &credential(), conceal)
}

fn invalid_disclosures(result: Result<Value, JoseError>) -> bool {
    matches!(result, Err(JoseError::InvalidDisclosures { .. }))
}

#[test]
fn a_disclosure_that_matches_no_digest_refuses_the_whole_token() {
    let key = key();
    let stray = Disclosure::new("stray", json!("x")).expect("builds");
    let token = forge_sd(&key, &sd_credential_header(&key), &credential(), &[stray]);

    assert!(invalid_disclosures(verify(&token, &key)));
}

#[test]
fn an_altered_disclosure_is_an_unused_disclosure() {
    let key = key();
    let token = concealing(&key, &["validFrom"]);
    let genuine = SdJwt::parse(&token).expect("parses");
    let altered = Disclosure::new("validFrom", json!("1999-01-01T00:00:00Z")).expect("builds");
    let swapped = SdJwt::new(genuine.jwt().to_owned(), vec![altered]).serialise();

    assert!(verify(&token, &key).is_ok());
    assert!(invalid_disclosures(verify(&swapped, &key)));
}

#[test]
fn a_disclosure_that_would_overwrite_a_claim_in_the_clear_is_refused() {
    let key = key();
    let token = concealing(&key, &["validFrom"]);
    let parsed = SdJwt::parse(&token).expect("parses");
    // The same name, in the clear and in a Disclosure the digest of which the token holds.
    let mut payload: Value = serde_json::from_slice(&base64_decode(
        parsed.jwt().split('.').nth(1).expect("a payload"),
    ))
    .expect("JSON");
    payload["validFrom"] = json!("2025-01-01T00:00:00Z");
    let forged = forge_sd(
        &key,
        &sd_credential_header(&key),
        &payload,
        parsed.disclosures(),
    );

    assert!(invalid_disclosures(verify(&forged, &key)));
}

fn base64_decode(text: &str) -> Vec<u8> {
    use base64::Engine as _;
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text)
        .expect("base64url")
}

#[test]
fn a_hash_algorithm_that_is_not_sha_256_is_refused() {
    let key = key();
    let mut payload = credential();
    payload["_sd_alg"] = json!("sha-512");

    let token = forge_sd(&key, &sd_credential_header(&key), &payload, &[]);

    assert!(invalid_disclosures(verify(&token, &key)));
}
