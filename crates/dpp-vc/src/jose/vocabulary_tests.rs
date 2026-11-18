//! The rule on top-level members: refused at signing unless declared, ignored at
//! verification.
//!
//! The lists themselves are compared with the published context by a test in
//! `dpp-tests`, so they cannot drift from it.

use serde_json::json;

use dpp_crypto::jws::KeyAlgorithm;

use super::test_fixtures::{KID, credential, generated, presentation};
use super::{
    CREDENTIAL_TERMS, JoseError, PRESENTATION_TERMS, sign_credential, sign_presentation,
    verify_credential, verify_presentation,
};

fn key() -> dpp_crypto::keystore::KeyEntry {
    generated(KeyAlgorithm::Ed25519)
}

#[test]
fn every_base_term_signs() {
    let mut document = credential();
    for term in CREDENTIAL_TERMS {
        document
            .as_object_mut()
            .expect("an object")
            .entry(term)
            .or_insert(json!("x"));
    }
    assert!(sign_credential(&document, &key(), KID, &[]).is_ok());

    let mut document = presentation(Vec::new());
    for term in PRESENTATION_TERMS {
        document
            .as_object_mut()
            .expect("an object")
            .entry(term)
            .or_insert(json!("x"));
    }
    assert!(sign_presentation(&document, &key(), KID, &[]).is_ok());
}

#[test]
fn an_undeclared_member_is_refused_and_named() {
    let mut document = credential();
    document["zeta"] = json!(1);
    document["alpha"] = json!(2);

    assert_eq!(
        sign_credential(&document, &key(), KID, &[]),
        Err(JoseError::UndeclaredMembers {
            names: vec!["alpha".to_owned(), "zeta".to_owned()]
        })
    );
}

/// Declaring is by exact name, and only the declared ones are let through.
#[test]
fn a_declared_extension_term_signs_and_an_undeclared_one_beside_it_does_not() {
    let mut document = credential();
    document["productPassport"] = json!({"id": "urn:example:1"});
    document["other"] = json!(1);
    let key = key();

    assert_eq!(
        sign_credential(&document, &key, KID, &["productPassport"]),
        Err(JoseError::UndeclaredMembers {
            names: vec!["other".to_owned()]
        })
    );
    let token =
        sign_credential(&document, &key, KID, &["productPassport", "other"]).expect("signs");
    assert_eq!(
        verify_credential(&token, &key.public_key(), None).expect("verifies"),
        document
    );
}

#[test]
fn the_jwt_registered_claim_names_need_no_declaration() {
    let mut document = credential();
    for name in ["iss", "sub", "aud", "exp", "nbf", "iat", "jti"] {
        document[name] = json!("https://issuer.example");
    }
    document["exp"] = json!(1_900_000_000);
    document["nbf"] = json!(1_700_000_000);
    document["iat"] = json!("2025-01-01T00:00:00Z");

    assert!(sign_credential(&document, &key(), KID, &[]).is_ok());

    let mut holder = presentation(Vec::new());
    holder["iss"] = json!("https://holder.example");
    holder["jti"] = json!("urn:uuid:1");
    assert!(sign_presentation(&holder, &key(), KID, &[]).is_ok());
}

/// `vc` and `vp` are refused whether or not the caller declares them, in a
/// credential and in a presentation.
#[test]
fn vc_and_vp_are_refused_even_when_declared() {
    for name in ["vc", "vp"] {
        let expected = Err(JoseError::ReservedClaim {
            name: if name == "vc" { "vc" } else { "vp" },
        });
        let mut document = credential();
        document[name] = json!({});
        assert_eq!(sign_credential(&document, &key(), KID, &[]), expected);
        assert_eq!(sign_credential(&document, &key(), KID, &[name]), expected);

        let mut holder = presentation(Vec::new());
        holder[name] = json!({});
        assert_eq!(sign_presentation(&holder, &key(), KID, &[]), expected);
        assert_eq!(sign_presentation(&holder, &key(), KID, &[name]), expected);
    }
}

/// Each document type has its own vocabulary: a credential's terms are not a
/// presentation's.
#[test]
fn a_presentation_term_in_a_credential_is_undeclared_and_the_other_way_round() {
    let mut document = credential();
    document["holder"] = json!("https://holder.example");
    assert_eq!(
        sign_credential(&document, &key(), KID, &[]),
        Err(JoseError::UndeclaredMembers {
            names: vec!["holder".to_owned()]
        })
    );

    let mut holder = presentation(Vec::new());
    holder["issuer"] = json!("https://issuer.example");
    holder["name"] = json!("A presentation");
    assert_eq!(
        sign_presentation(&holder, &key(), KID, &[]),
        Err(JoseError::UndeclaredMembers {
            names: vec!["issuer".to_owned(), "name".to_owned()]
        })
    );
    assert!(sign_presentation(&holder, &key(), KID, &["issuer", "name"]).is_ok());
}

/// Verification does not read the vocabulary: a token whose payload has members no
/// signer here would have written is verified, and so is the presentation that
/// carries them (VC-JOSE-COSE §3.1.3, §5.4).
#[test]
fn verification_ignores_members_it_does_not_understand() {
    let key = key();
    let mut document = credential();
    document["badExtension"] = json!({"huh": "what?"});
    document["anotherBadOne"] = json!([1, 2]);
    let forged = super::test_fixtures::forge(
        &key,
        &super::test_fixtures::credential_header(&key),
        &document,
    );
    assert!(sign_credential(&document, &key, KID, &[]).is_err());

    let verified = verify_credential(&forged, &key.public_key(), None).expect("verifies");

    assert_eq!(verified["badExtension"], json!({"huh": "what?"}));

    let mut holder = presentation(Vec::new());
    holder["unknownToTheVocabulary"] = json!(true);
    let forged = super::test_fixtures::forge(
        &key,
        &super::test_fixtures::presentation_header(&key),
        &holder,
    );
    assert!(
        verify_presentation(&forged, &key.public_key(), None, |_, _, _| {
            super::EnvelopeDecision::Unknown
        })
        .is_ok()
    );
}
