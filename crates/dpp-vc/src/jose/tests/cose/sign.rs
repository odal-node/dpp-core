//! Signing a credential or a presentation with COSE, and the `kid` rule that only signers keep
//! (VC-JOSE-COSE §3.3.1, §3.3.2, §4.2).
//!
//! Messages a signer would never write are forged with a genuine signature
//! ([`super::super::cose::common`]), so a refusal is the rule it names and not a signature that never
//! verified.

use serde_json::json;

use super::super::super::{
    JoseError, VerifyOptions, sign_credential, sign_credential_cose, sign_credential_sd_jwt,
    sign_presentation_cose, verify_credential, verify_credential_cose, verify_presentation_cose,
};
use super::super::cose::common::{
    ALGORITHMS, Header, THUMBPRINT, cred_header, forge_document, forged_credential, holder, issuer,
    verify_vc,
};
use crate::test_support::{
    KID, NOT_REQUIRED, cose_envelope, credential, forge as forge_jws, presentation, public_key,
    signing_key,
};
use dpp_crypto::cose::{ContentType, verify_sign1};

// ── signing ───────────────────────────────────────────────────────────────────

#[test]
fn a_credential_and_a_presentation_round_trip_under_every_algorithm() {
    for algorithm in ALGORITHMS {
        let key = signing_key(algorithm);
        let public = public_key(algorithm);

        let signed = sign_credential_cose(&credential(), &key, KID, &[]).expect(algorithm);
        assert_eq!(
            verify_credential_cose(&signed, &public, VerifyOptions::new()),
            Ok(credential()),
            "{algorithm}"
        );

        let document = presentation(vec![cose_envelope(&signed)]);
        let signed = sign_presentation_cose(&document, &key, KID, &[]).expect(algorithm);
        let verified = verify_presentation_cose(&signed, &public, VerifyOptions::new())
            .unwrap_or_else(|e| panic!("{algorithm}: {e}"));
        assert_eq!(verified.payload(), &document, "{algorithm}");
    }
}

/// §3.3.1: the unsecured credential is the unencoded `COSE_Sign1` payload, `typ` is
/// `application/vc+cose` and the content type `application/vc`. The `kid` is the UTF-8 bytes of
/// the identifier given, and no CWT claim is written.
#[test]
fn the_signer_writes_the_document_the_headers_and_no_claims() {
    for (signed, typ, cty, document) in [
        (
            sign_credential_cose(&credential(), &issuer(), KID, &[]).unwrap(),
            "application/vc+cose",
            "application/vc",
            credential(),
        ),
        (
            sign_presentation_cose(&presentation(vec![]), &holder(), KID, &[]).unwrap(),
            "application/vp+cose",
            "application/vp",
            presentation(vec![]),
        ),
    ] {
        assert_eq!(signed[0], 0xd2, "tagged with 18");
        let key = if typ.starts_with("application/vc") {
            public_key("p256")
        } else {
            public_key("p384")
        };
        let verified = verify_sign1(&signed, &key).expect("verifies");
        assert_eq!(verified.typ, Some(ContentType::Text(typ.to_owned())));
        assert_eq!(
            verified.content_type,
            Some(ContentType::Text(cty.to_owned()))
        );
        assert_eq!(verified.kid.as_deref(), Some(KID.as_bytes()));
        assert_eq!(verified.cwt_claims, dpp_crypto::cose::CwtClaims::default());
        assert_eq!(verified.payload, serde_jcs::to_vec(&document).unwrap());
    }
}

#[test]
fn something_that_is_not_an_object_is_refused() {
    assert_eq!(
        sign_credential_cose(&json!([]), &issuer(), KID, &[]),
        Err(JoseError::NotAnObject { what: "credential" })
    );
    assert_eq!(
        sign_presentation_cose(&json!("p"), &holder(), KID, &[]),
        Err(JoseError::NotAnObject {
            what: "presentation"
        })
    );
}

/// RFC 8785 writes every number as an IEEE 754 double, so an integer beyond ±(2^53 − 1) would
/// be signed as a different number. Both COSE signers refuse one, nested or not, as the
/// `vc+jwt` signer does, and the largest exact integer signs and reads back unchanged.
#[test]
fn an_integer_a_double_cannot_hold_exactly_is_refused_at_signing() {
    const INEXACT: u64 = 9_007_199_254_740_993;
    const EXACT: u64 = 9_007_199_254_740_991;
    fn refused<T>(result: Result<T, JoseError>) -> bool {
        matches!(result, Err(JoseError::Signing { .. }))
    }

    let mut document = credential();
    document["credentialSubject"]["serialNumber"] = json!(INEXACT);
    assert!(refused(sign_credential_cose(
        &document,
        &issuer(),
        KID,
        &[]
    )));
    assert!(refused(sign_credential(&document, &issuer(), KID, &[])));

    let envelope =
        cose_envelope(&sign_credential_cose(&credential(), &issuer(), KID, &[]).expect("signs"));
    let mut document = presentation(vec![envelope]);
    document["batch"] = json!({ "serial": [INEXACT] });
    assert!(refused(sign_presentation_cose(
        &document,
        &holder(),
        KID,
        &["batch"]
    )));

    let mut document = credential();
    document["credentialSubject"]["serialNumber"] = json!(EXACT);
    let signed = sign_credential_cose(&document, &issuer(), KID, &[]).expect("exact");
    assert_eq!(verify_vc(&signed), Ok(document));
}

/// An undeclared member is refused at signing, as for the other forms, and a declared one signs.
#[test]
fn an_undeclared_member_is_refused_unless_it_is_declared() {
    let mut document = credential();
    document["flavour"] = json!("plain");

    assert_eq!(
        sign_credential_cose(&document, &issuer(), KID, &[]),
        Err(JoseError::UndeclaredMembers {
            names: vec!["flavour".to_owned()]
        })
    );
    let signed = sign_credential_cose(&document, &issuer(), KID, &["flavour"]).expect("declared");
    assert_eq!(verify_vc(&signed), Ok(document));
}

/// The payload is not a JWT Claims Set. The registered claim names that the JOSE signer allows
/// without a declaration are undeclared terms here, and each is refused.
#[test]
fn the_jwt_registered_claim_names_are_undeclared_terms_in_a_cose_payload() {
    for (name, value) in [
        ("iss", json!("https://issuer.example")),
        ("sub", json!("s")),
        ("aud", json!("https://verifier.example")),
        ("exp", json!(2_000_000_000)),
        ("nbf", json!(1)),
        ("iat", json!(1)),
        ("jti", json!("j")),
    ] {
        let mut document = credential();
        document[name] = value;

        // The control: the JOSE signer allows it.
        assert!(
            sign_credential(&document, &issuer(), KID, &[]).is_ok(),
            "{name} is allowed in a JWT Claims Set"
        );
        assert_eq!(
            sign_credential_cose(&document, &issuer(), KID, &[]),
            Err(JoseError::UndeclaredMembers {
                names: vec![name.to_owned()]
            }),
            "{name}"
        );
    }
}

/// VC-JOSE-COSE §1.1.2.1 forbids `vc` and `vp` in "any JWT Claims Set". A COSE payload is not one,
/// so a `vc` member is refused only as an undeclared term, and signs when it is declared.
#[test]
fn vc_and_vp_members_are_ordinary_terms_in_a_cose_payload() {
    for name in ["vc", "vp"] {
        let mut document = credential();
        document[name] = json!({"x": 1});

        // The JOSE signer refuses it even when declared.
        assert!(matches!(
            sign_credential(&document, &issuer(), KID, &[name]),
            Err(JoseError::ReservedClaim { .. })
        ));
        assert_eq!(
            sign_credential_cose(&document, &issuer(), KID, &[]),
            Err(JoseError::UndeclaredMembers {
                names: vec![name.to_owned()]
            })
        );
        let signed = sign_credential_cose(&document, &issuer(), KID, &[name]).expect("declared");
        // And a verifier does not refuse it: the payload is not a claims set.
        assert_eq!(verify_vc(&signed), Ok(document), "{name}");
    }
}

#[test]
fn the_properties_a_credential_and_a_presentation_must_carry_are_checked() {
    let mut no_context = credential();
    no_context.as_object_mut().unwrap().remove("@context");
    let mut no_subject = credential();
    no_subject
        .as_object_mut()
        .unwrap()
        .remove("credentialSubject");
    let mut not_a_credential = credential();
    not_a_credential["type"] = json!(["Other"]);

    assert!(matches!(
        sign_credential_cose(&no_context, &issuer(), KID, &[]),
        Err(JoseError::InvalidContext { .. })
    ));
    assert!(matches!(
        sign_credential_cose(&no_subject, &issuer(), KID, &[]),
        Err(JoseError::InvalidCredentialSubject { .. })
    ));
    assert!(matches!(
        sign_credential_cose(&not_a_credential, &issuer(), KID, &[]),
        Err(JoseError::InvalidType { .. })
    ));
    let mut bad_holder = presentation(vec![]);
    bad_holder["holder"] = json!("not a url");
    assert!(matches!(
        sign_presentation_cose(&bad_holder, &holder(), KID, &[]),
        Err(JoseError::InvalidHolder { .. })
    ));
    assert!(matches!(
        sign_presentation_cose(&credential(), &holder(), KID, &[]),
        Err(JoseError::UndeclaredMembers { .. } | JoseError::InvalidType { .. })
    ));
}

/// A presentation's envelopes are given already enveloped and are not opened at signing, so the
/// suite's row 28 input, whose envelope is not a JWS, signs.
#[test]
fn the_envelopes_of_a_presentation_are_not_opened_at_signing() {
    let document = presentation(vec![json!({
        "@context": "https://www.w3.org/ns/credentials/v2",
        "type": "EnvelopedVerifiableCredential",
        "id": "data:application/vc+jwt,not-a-jws",
    })]);

    assert!(sign_presentation_cose(&document, &holder(), KID, &[]).is_ok());
}

// ── the kid: held by signers and not by verifiers ─────────────────────────────

/// §4.2 opens "When using controlled identifier documents with this specification", and §5
/// assumes the key was found. A verifier here is given the key, so a bare `kid` with no `iss`
/// verifies in every form; a signer keeps refusing it, so the tokens this crate writes stay
/// strict.
#[test]
fn a_bare_kid_verifies_and_is_refused_at_signing_in_every_form() {
    let key = issuer();
    let public = key.public_key();
    let refused = |kid: &str| JoseError::KidNotAbsolute {
        kid: kid.to_owned(),
    };

    // vc+cose
    assert_eq!(
        sign_credential_cose(&credential(), &key, THUMBPRINT, &[]),
        Err(refused(THUMBPRINT))
    );
    assert!(
        verify_vc(&forged_credential(
            &credential(),
            &Header::credential(THUMBPRINT)
        ))
        .is_ok()
    );
    let mut without_kid = cred_header();
    without_kid.kid = None;
    assert!(verify_vc(&forged_credential(&credential(), &without_kid)).is_ok());

    // vc+jwt
    assert_eq!(
        sign_credential(&credential(), &key, THUMBPRINT, &[]),
        Err(refused(THUMBPRINT))
    );
    let header = json!({"alg": "ES256", "kid": THUMBPRINT, "typ": "vc+jwt", "cty": "vc"});
    let token = forge_jws(&key, &header, &credential());
    assert!(verify_credential(&token, &public, VerifyOptions::new()).is_ok());

    // vc+sd-jwt
    assert_eq!(
        sign_credential_sd_jwt(&credential(), &key, THUMBPRINT, &[], &[], None).map(|_| ()),
        Err(refused(THUMBPRINT))
    );
    let sd_header = json!({"alg": "ES256", "kid": THUMBPRINT, "typ": "vc+sd-jwt", "cty": "vc"});
    let sd_token = format!("{}~", forge_jws(&key, &sd_header, &credential()));
    assert!(
        super::super::super::verify_credential_sd_jwt(
            &sd_token,
            &public,
            VerifyOptions::new(),
            &NOT_REQUIRED
        )
        .is_ok()
    );

    // And the presentations, whose holder is a URL.
    let holder_key = holder();
    assert_eq!(
        sign_presentation_cose(&presentation(vec![]), &holder_key, THUMBPRINT, &[]),
        Err(refused(THUMBPRINT))
    );
    assert!(
        verify_presentation_cose(
            &forge_document(
                &holder_key,
                &Header::presentation(THUMBPRINT),
                &presentation(vec![])
            ),
            &public_key("p384"),
            VerifyOptions::new()
        )
        .is_ok()
    );
}

/// An `iss` member of a COSE document is a term like any other, and exempts nothing: the
/// signer writes no `iss` claim, so the `kid` rule always applies.
#[test]
fn an_iss_member_does_not_exempt_a_cose_signer_from_the_kid_rule() {
    let mut document = credential();
    document["iss"] = json!("https://issuer.example");

    assert!(matches!(
        sign_credential_cose(&document, &issuer(), THUMBPRINT, &["iss"]),
        Err(JoseError::KidNotAbsolute { .. })
    ));
    assert!(sign_credential_cose(&document, &issuer(), KID, &["iss"]).is_ok());
}
