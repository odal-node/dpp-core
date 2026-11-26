//! Verifying a credential: each check refuses on its own, with a control that
//! passes.
//!
//! Tokens are forged: a genuine signature over a header or payload that
//! `sign_credential` would never write, so a refusal can only be the check it names.
//! The time claims are in `temporal.rs`, `iss` and the `kid` in `issuer.rs`, and
//! the suite's inputs in `suite/verification.rs`.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;

use super::super::VerifyOptions;
use super::super::{JoseError, sign_credential, verify_credential};
use crate::test_support::{
    KID, credential, credential_header, forge, forge_raw, generated, with_credential, with_header,
};

fn key() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP256)
}

fn verify(token: &str, key: &KeyEntry) -> Result<Value, JoseError> {
    verify_credential(token, &key.public_key(), VerifyOptions::new())
}

/// The control: the forgery with nothing wrong verifies, so every refusal below is
/// the thing changed.
#[test]
fn a_forged_credential_with_nothing_wrong_verifies() {
    let key = key();

    assert_eq!(
        verify(&forge(&key, &credential_header(&key), &credential()), &key),
        Ok(credential())
    );
}

// ── the compact form and the header ───────────────────────────────────────────

#[test]
fn something_that_is_not_three_segments_is_not_a_compact_jws() {
    let key = key();
    for text in ["", "abc", "a.b", "a.b.c.d", "{\"a\": 1}"] {
        assert_eq!(
            verify(text, &key),
            Err(JoseError::NotCompactJws),
            "{text:?}"
        );
    }
}

#[test]
fn an_unreadable_header_is_refused() {
    let key = key();
    for text in ["!!!.e30.AA", "W10.e30.AA", "bm90IGpzb24.e30.AA"] {
        assert_eq!(
            verify(text, &key),
            Err(JoseError::UnreadableHeader),
            "{text}"
        );
    }
}

#[test]
fn crit_is_refused_and_the_same_token_without_it_verifies() {
    let key = key();
    let crit = with_header(&key, |h| {
        h["crit"] = json!(["x"]);
        h["x"] = json!(1);
    });
    let control = with_header(&key, |h| {
        h["x"] = json!(1);
    });

    assert_eq!(verify(&crit, &key), Err(JoseError::UnsupportedCritical));
    assert!(verify(&control, &key).is_ok());
}

/// RFC 7797 §7: a JWT MUST NOT use `b64` with `false`. The token is validly signed, so
/// the refusal is the header's, and it comes after `crit`: a header with both is
/// refused as critical.
#[test]
fn b64_false_is_refused_and_crit_is_reported_first_when_both_are_present() {
    let key = key();
    let unencoded = with_header(&key, |h| h["b64"] = json!(false));
    let critical_and_unencoded = with_header(&key, |h| {
        h["crit"] = json!(["b64"]);
        h["b64"] = json!(false);
    });

    assert_eq!(verify(&unencoded, &key), Err(JoseError::UnencodedPayload));
    assert_eq!(
        verify(&critical_and_unencoded, &key),
        Err(JoseError::UnsupportedCritical)
    );
}

#[test]
fn alg_none_is_refused() {
    let key = key();
    let none = with_header(&key, |h| h["alg"] = json!("none"));
    let unsecured = {
        use base64::Engine as _;
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
        format!(
            "{}.{}.",
            b64.encode(br#"{"alg":"none","typ":"vc+jwt"}"#),
            b64.encode(serde_json::to_vec(&credential()).expect("serialises"))
        )
    };

    for token in [none, unsecured] {
        assert_eq!(
            verify(&token, &key),
            Err(JoseError::AlgorithmMismatch {
                header_alg: Some("none".to_owned()),
                key_alg: KeyAlgorithm::EcdsaP256,
            })
        );
    }
}

#[test]
fn an_absent_or_non_string_alg_is_refused() {
    let key = key();
    for change in [json!(null), json!(256)] {
        let token = with_header(&key, |h| h["alg"] = change.clone());
        assert_eq!(
            verify(&token, &key),
            Err(JoseError::AlgorithmMismatch {
                header_alg: None,
                key_alg: KeyAlgorithm::EcdsaP256,
            })
        );
    }
    let absent = with_header(&key, |h| {
        h.as_object_mut().expect("an object").remove("alg");
    });
    assert!(matches!(
        verify(&absent, &key),
        Err(JoseError::AlgorithmMismatch {
            header_alg: None,
            ..
        })
    ));
}

#[test]
fn unknown_header_parameters_are_ignored() {
    let key = key();
    let token = with_header(&key, |h| {
        h["x"] = json!(1);
        h["jku"] = json!("https://elsewhere.example/keys");
    });

    assert!(verify(&token, &key).is_ok());
}

// ── the signature ─────────────────────────────────────────────────────────────

#[test]
fn a_token_signed_by_another_key_is_refused() {
    let signer = key();
    let token = forge(&signer, &credential_header(&signer), &credential());

    assert_eq!(
        verify(&token, &key()),
        Err(JoseError::SignatureDoesNotVerify)
    );
}

#[test]
fn a_signature_that_cannot_be_checked_is_unverifiable() {
    let key = key();
    let token = with_header(&key, |_| {});
    let (signing_input, _) = token.rsplit_once('.').expect("three segments");

    for signature in ["AA", "!!!", ""] {
        let refused = verify(&format!("{signing_input}.{signature}"), &key).expect_err("refused");
        assert!(
            matches!(refused, JoseError::Unverifiable { .. }),
            "{signature:?}: {refused:?}"
        );
    }
}

/// A changed payload under the same signature does not verify.
#[test]
fn a_payload_changed_after_signing_is_refused() {
    let key = key();
    let token = sign_credential(&credential(), &key, KID, &[]).expect("signs");
    let other = sign_credential(
        &json!({
            "@context": ["https://www.w3.org/ns/credentials/v2"],
            "type": ["VerifiableCredential"],
            "issuer": "https://other.example",
            "credentialSubject": {"id": "did:example:other", "name": "Other"},
        }),
        &key,
        KID,
        &[],
    )
    .expect("signs");
    let spliced = format!(
        "{}.{}.{}",
        token.split('.').next().expect("header"),
        other.split('.').nth(1).expect("payload"),
        token.split('.').nth(2).expect("signature")
    );

    assert_eq!(
        verify(&spliced, &key),
        Err(JoseError::SignatureDoesNotVerify)
    );
}

// ── `typ` and `cty` ───────────────────────────────────────────────────────────

#[test]
fn typ_is_required() {
    let key = key();
    let token = with_header(&key, |h| {
        h.as_object_mut().expect("an object").remove("typ");
    });

    assert_eq!(
        verify(&token, &key),
        Err(JoseError::WrongMediaType {
            member: "typ",
            found: None,
            expected: "vc+jwt",
        })
    );
}

#[test]
fn typ_names_vc_jwt_with_or_without_the_application_prefix_in_any_case() {
    let key = key();
    for typ in [
        "vc+jwt",
        "application/vc+jwt",
        "VC+JWT",
        "Application/VC+JWT",
    ] {
        let token = with_header(&key, |h| h["typ"] = json!(typ));
        assert!(verify(&token, &key).is_ok(), "{typ}");
    }
}

#[test]
fn another_typ_is_refused() {
    let key = key();
    for (typ, found) in [
        (json!("vp+jwt"), "\"vp+jwt\""),
        (json!("JWT"), "\"JWT\""),
        (json!("dc+sd-jwt"), "\"dc+sd-jwt\""),
        (json!(7), "7"),
    ] {
        let token = with_header(&key, |h| h["typ"] = typ.clone());
        assert_eq!(
            verify(&token, &key),
            Err(JoseError::WrongMediaType {
                member: "typ",
                found: Some(found.to_owned()),
                expected: "vc+jwt",
            })
        );
    }
}

#[test]
fn cty_is_optional_and_when_present_names_vc() {
    let key = key();
    let absent = with_header(&key, |h| {
        h.as_object_mut().expect("an object").remove("cty");
    });
    let prefixed = with_header(&key, |h| h["cty"] = json!("application/vc"));
    let wrong = with_header(&key, |h| h["cty"] = json!("vp"));

    assert!(verify(&absent, &key).is_ok());
    assert!(verify(&prefixed, &key).is_ok());
    assert_eq!(
        verify(&wrong, &key),
        Err(JoseError::WrongMediaType {
            member: "cty",
            found: Some("\"vp\"".to_owned()),
            expected: "vc",
        })
    );
}

// ── the payload ───────────────────────────────────────────────────────────────

#[test]
fn a_payload_that_is_not_a_json_object_is_refused() {
    let key = key();
    let header = credential_header(&key);
    for segment in ["!!!", "bm90IGpzb24", "W10", "Ng"] {
        let token = forge_raw(&key, &header, segment);
        assert!(
            matches!(
                verify(&token, &key),
                Err(JoseError::MalformedPayload { .. })
            ),
            "{segment}"
        );
    }
}

#[test]
fn a_payload_without_a_usable_context_is_refused() {
    let key = key();
    let absent = with_credential(&key, |c| {
        c.as_object_mut().expect("an object").remove("@context");
    });
    let number = with_credential(&key, |c| c["@context"] = json!(7));
    let string = with_credential(&key, |c| {
        c["@context"] = json!("https://www.w3.org/ns/credentials/v2")
    });

    assert!(matches!(
        verify(&absent, &key),
        Err(JoseError::InvalidContext { .. })
    ));
    assert!(matches!(
        verify(&number, &key),
        Err(JoseError::InvalidContext { .. })
    ));
    assert!(verify(&string, &key).is_ok());
}

#[test]
fn vc_and_vp_claims_are_refused_each_on_its_own() {
    let key = key();
    for name in ["vc", "vp"] {
        let token = with_credential(&key, |c| c[name] = json!({}));
        assert_eq!(
            verify(&token, &key),
            Err(JoseError::ReservedClaim {
                name: if name == "vc" { "vc" } else { "vp" }
            })
        );
    }
}

/// §3.1.3: `iat` describes the signature and the suite's credentials carry it as an
/// RFC 3339 string. It is never read, so a string, a number and nonsense all pass.
#[test]
fn iat_in_any_form_is_accepted() {
    let key = key();
    for iat in [
        json!("2010-01-01T19:23:24Z"),
        json!(1_700_000_000),
        json!("never"),
        json!(null),
    ] {
        let token = with_credential(&key, |c| c["iat"] = iat.clone());
        assert_eq!(
            verify_credential(
                &token,
                &key.public_key(),
                VerifyOptions::new().at(2_000_000_000)
            )
            .map(|_| ()),
            Ok(()),
            "{iat}"
        );
    }
}
