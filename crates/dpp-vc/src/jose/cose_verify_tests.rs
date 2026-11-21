//! Verifying a COSE message: `typ` and the content type, which are held to the policy of the
//! JOSE form, and the payload (VC-JOSE-COSE §3.3.1, §5.3).
//!
//! Messages a signer would never write are forged with a genuine signature
//! ([`super::cose_support`]), so a refusal is the rule it names and not a signature that never
//! verified.

use serde_json::json;

use super::cose_support::{
    cred_header, forge, forged_credential, issuer, uint, verify_vc, verify_vc_with,
};
use super::{
    JoseError, VerifyOptions, sign_credential_cose, sign_presentation_cose, verify_credential,
    verify_credential_cose, verify_presentation_cose,
};
use crate::test_support::{KID, credential, forge as forge_jws, presentation, public_key};
use dpp_crypto::cose::CoseError;

// ── typ and the content type: the policy of the JOSE form ────────────────────

/// `typ` and the content type are held to the policy the `vc+jwt` form applies to `typ` and
/// `cty`: presence, the value, and the `application/` prefix. Each combination gets the same
/// verdict in both, though §3.3.1 says SHOULD for both values.
#[test]
fn typ_and_content_type_are_held_to_the_policy_of_the_jose_form() {
    let typs = |base: &str| -> Vec<Option<String>> {
        vec![
            None,
            Some(base.to_owned()),
            Some(format!("application/{base}")),
            Some(format!("Application/{}", base.to_uppercase())),
            Some(format!("text/{base}")),
            Some("really/bad".to_owned()),
            Some(String::new()),
        ]
    };
    let ctys = |base: &str| -> Vec<Option<String>> {
        vec![
            None,
            Some(base.to_owned()),
            Some(format!("application/{base}")),
            Some(base.to_uppercase()),
            Some("application/vp".to_owned()),
            Some("very/bad".to_owned()),
        ]
    };
    let jose_key = issuer();
    let mut seen = (0, 0);
    for typ in typs("vc+jwt").into_iter().zip(typs("vc+cose")) {
        for cty in ctys("vc").into_iter().zip(ctys("vc")) {
            let mut header = json!({"alg": "ES256", "kid": KID});
            if let Some(typ) = &typ.0 {
                header["typ"] = json!(typ);
            }
            if let Some(cty) = &cty.0 {
                header["cty"] = json!(cty);
            }
            let jose = verify_credential(
                &forge_jws(&jose_key, &header, &credential()),
                &public_key("p256"),
                VerifyOptions::new(),
            );

            let mut cose_header = cred_header().without_typ().without_cty();
            if let Some(typ) = &typ.1 {
                cose_header = cose_header.typ(typ);
            }
            if let Some(cty) = &cty.1 {
                cose_header = cose_header.cty(cty);
            }
            let cose = verify_vc(&forged_credential(&credential(), &cose_header));

            match (&jose, &cose) {
                (Ok(_), Ok(_)) => seen.0 += 1,
                (
                    Err(JoseError::WrongMediaType {
                        member: a,
                        found: fa,
                        ..
                    }),
                    Err(JoseError::WrongMediaType {
                        member: b,
                        found: fb,
                        ..
                    }),
                ) => {
                    // The same value, but for the name of the form it was written for.
                    let name = |found: &Option<String>, form: &str| {
                        found
                            .as_ref()
                            .map(|f| f.to_lowercase().replace(form, "FORM"))
                    };
                    assert_eq!(
                        (a, name(fa, "vc+jwt")),
                        (b, name(fb, "vc+cose")),
                        "{typ:?} {cty:?}"
                    );
                    seen.1 += 1;
                }
                other => panic!("{typ:?} {cty:?}: the forms disagree: {other:?}"),
            }
        }
    }
    // A count, because a loop that never compares passes.
    assert_eq!(seen.0 + seen.1, 42);
    assert!(seen.0 >= 4 && seen.1 >= 20, "{seen:?}");
}

#[test]
fn the_media_type_refusal_names_the_member_and_what_it_found() {
    let wrong_typ = forged_credential(&credential(), &cred_header().typ("really/bad"));
    let wrong_cty = forged_credential(&credential(), &cred_header().cty("very/bad"));
    let no_typ = forged_credential(&credential(), &cred_header().without_typ());
    let coap = {
        let mut header = cred_header();
        header.typ = Some(uint(5));
        forged_credential(&credential(), &header)
    };

    assert_eq!(
        verify_vc(&wrong_typ),
        Err(JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"really/bad\"".to_owned()),
            expected: "vc+cose"
        })
    );
    assert_eq!(
        verify_vc(&wrong_cty),
        Err(JoseError::WrongMediaType {
            member: "cty",
            found: Some("\"very/bad\"".to_owned()),
            expected: "vc"
        })
    );
    assert_eq!(
        verify_vc(&no_typ),
        Err(JoseError::WrongMediaType {
            member: "typ",
            found: None,
            expected: "vc+cose"
        })
    );
    assert_eq!(
        verify_vc(&coap),
        Err(JoseError::WrongMediaType {
            member: "typ",
            found: Some("5".to_owned()),
            expected: "vc+cose"
        })
    );
}

/// A credential is not taken for a presentation, or the reverse.
#[test]
fn a_credential_is_not_a_presentation() {
    let credential_message = sign_credential_cose(&credential(), &issuer(), KID, &[]).unwrap();
    let presentation_message =
        sign_presentation_cose(&presentation(vec![]), &issuer(), KID, &[]).unwrap();

    assert!(matches!(
        verify_presentation_cose(
            &credential_message,
            &public_key("p256"),
            VerifyOptions::new()
        ),
        Err(JoseError::WrongMediaType { member: "typ", .. })
    ));
    assert!(matches!(
        verify_vc(&presentation_message),
        Err(JoseError::WrongMediaType { member: "typ", .. })
    ));
}

// ── the message and the payload ───────────────────────────────────────────────

#[test]
fn a_message_that_does_not_verify_is_refused_by_the_cose_layer() {
    let signed = sign_credential_cose(&credential(), &issuer(), KID, &[]).unwrap();
    let mut tampered = signed.clone();
    let last = tampered.len() - 1;
    tampered[last] ^= 1;

    assert_eq!(
        verify_vc(&tampered),
        Err(JoseError::Cose(CoseError::Signature))
    );
    assert!(matches!(
        verify_vc(b"not cose"),
        Err(JoseError::Cose(CoseError::Malformed(_)))
    ));
    assert_eq!(
        verify_credential_cose(&signed, &public_key("p384"), VerifyOptions::new()),
        Err(JoseError::Cose(CoseError::AlgorithmMismatch))
    );
    // The untagged form is read as well.
    assert!(verify_vc(&signed[1..]).is_ok());
}

#[test]
fn a_payload_that_is_not_a_json_object_is_refused() {
    for payload in [&b"not json"[..], b"[]", b"\"text\"", b"7", b""] {
        let message = forge(&issuer(), &cred_header(), payload);
        assert!(
            matches!(verify_vc(&message), Err(JoseError::MalformedPayload { .. })),
            "{payload:?}"
        );
    }
}

#[test]
fn the_payload_is_held_to_the_data_model() {
    let mut bad_context = credential();
    bad_context["@context"] = json!(["https://example.org/other"]);
    let mut no_subject = credential();
    no_subject
        .as_object_mut()
        .unwrap()
        .remove("credentialSubject");
    let mut bad_issuer = credential();
    bad_issuer["issuer"] = json!("Example Issuer Ltd");
    let mut not_a_credential = credential();
    not_a_credential["type"] = json!(["VerifiablePresentation"]);

    for (document, check) in [
        (
            bad_context,
            (|e| matches!(e, JoseError::InvalidContext { .. })) as fn(&JoseError) -> bool,
        ),
        (no_subject, |e| {
            matches!(e, JoseError::InvalidCredentialSubject { .. })
        }),
        (bad_issuer, |e| matches!(e, JoseError::InvalidIssuer { .. })),
        (not_a_credential, |e| {
            matches!(e, JoseError::InvalidType { .. })
        }),
    ] {
        let refused =
            verify_vc(&forged_credential(&document, &cred_header())).expect_err("refused");
        assert!(check(&refused), "{refused:?}");
    }
}

/// Members the verifier does not understand are ignored (§3.3.3, §5.4).
#[test]
fn unknown_members_are_ignored() {
    let mut document = credential();
    document["somethingNew"] = json!({"x": 1});

    assert_eq!(
        verify_vc(&forged_credential(&document, &cred_header())),
        Ok(document)
    );
}

/// The payload is not a claims set: an `exp`, `aud` or `iss` member of it is a member, and
/// nothing is enforced from it. The claims are the CWT claims of the protected header.
#[test]
fn jwt_claim_names_in_the_payload_are_not_read() {
    let mut document = credential();
    document["exp"] = json!(1);
    document["nbf"] = json!(4_000_000_000_i64);
    document["aud"] = json!("https://nobody.example");
    document["iss"] = json!("https://other.example");

    let message = forged_credential(&document, &cred_header());

    assert_eq!(
        verify_vc_with(&message, VerifyOptions::new().at(2_000_000_000)),
        Ok(document)
    );
}
