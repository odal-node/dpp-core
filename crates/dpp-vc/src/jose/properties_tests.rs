//! The required properties of a credential and a presentation, and the holder's `kid`.
//!
//! Each refusal is shown twice: at signing, and on a token forged over the same
//! document. Signing refuses first, so the forged token is the only way to show that
//! verification refuses the same defect for the same reason. Each refusal has a
//! control that passes.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;

use super::{
    JoseError, sign_credential, sign_presentation, verify_credential, verify_presentation,
};
use crate::test_support::{
    KID, credential, credential_header, forge, generated, presentation, presentation_header,
};

const V2: &str = "https://www.w3.org/ns/credentials/v2";
const THUMBPRINT: &str = "73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU";

fn key() -> KeyEntry {
    generated(KeyAlgorithm::EcdsaP256)
}

/// `credential()` with `change` applied.
fn edited(change: impl FnOnce(&mut Value)) -> Value {
    let mut document = credential();
    change(&mut document);
    document
}

/// A credential refused at signing, and refused for the same reason when verified.
fn credential_refused(document: &Value) -> JoseError {
    let key = key();
    let signed = sign_credential(document, &key, KID, &[]).expect_err("refused at signing");
    let token = forge(&key, &credential_header(&key), document);
    let verified =
        verify_credential(&token, &key.public_key(), None).expect_err("refused at verification");
    assert_eq!(signed, verified);
    signed
}

/// A presentation refused at signing, and refused for the same reason when verified.
fn presentation_refused(document: &Value) -> JoseError {
    let key = key();
    let signed = sign_presentation(document, &key, KID, &[]).expect_err("refused at signing");
    let token = forge(&key, &presentation_header(&key), document);
    let verified =
        verify_presentation(&token, &key.public_key(), None).expect_err("refused at verification");
    assert_eq!(signed, verified);
    signed
}

/// A presentation's outer token, forged under `key` with `kid` in its header.
fn outer(key: &KeyEntry, kid: &str, document: &Value) -> String {
    let header =
        json!({"alg": key.algorithm().jose_alg(), "kid": kid, "typ": "vp+jwt", "cty": "vp"});
    forge(key, &header, document)
}

// ── @context (VC DM 2.0 §4.3) ─────────────────────────────────────────────────

#[test]
fn a_context_set_that_begins_with_the_v2_url_is_accepted() {
    let key = key();
    for context in [
        json!(V2),
        json!([V2]),
        json!([V2, "https://www.w3.org/ns/credentials/examples/v2"]),
        json!([V2, {"@vocab": "https://example.org/terms#"}]),
    ] {
        let document = edited(|c| c["@context"] = context.clone());

        assert!(
            sign_credential(&document, &key, KID, &[]).is_ok(),
            "{context}"
        );
        let token = forge(&key, &credential_header(&key), &document);
        assert!(
            verify_credential(&token, &key.public_key(), None).is_ok(),
            "{context}"
        );
    }
}

#[test]
fn a_context_set_that_does_not_begin_with_the_v2_url_is_refused() {
    for context in [
        json!([]),
        json!(["https://www.w3.org/ns/credentials/examples/v2"]),
        json!([{"@vocab": "https://example.org/terms#"}, V2]),
        json!("https://www.w3.org/ns/credentials/examples/v2"),
        json!({"@vocab": "https://example.org/terms#"}),
        json!(7),
    ] {
        let document = edited(|c| c["@context"] = context.clone());

        assert!(
            matches!(
                credential_refused(&document),
                JoseError::InvalidContext { .. }
            ),
            "{context}"
        );
    }
}

#[test]
fn an_absent_context_is_refused() {
    let document = edited(|c| {
        c.as_object_mut().expect("an object").remove("@context");
    });

    assert!(matches!(
        credential_refused(&document),
        JoseError::InvalidContext { .. }
    ));
}

#[test]
fn a_later_context_item_must_be_a_url_or_an_object() {
    for item in [json!(7), json!("not a url"), json!(null), json!(["nested"])] {
        let document = edited(|c| c["@context"] = json!([V2, item.clone()]));

        assert!(
            matches!(
                credential_refused(&document),
                JoseError::InvalidContext { .. }
            ),
            "{item}"
        );
    }
}

#[test]
fn a_presentation_context_is_held_to_the_same_rule() {
    let mut document = presentation(Vec::new());
    document["@context"] = json!("https://www.w3.org/ns/credentials/examples/v2");

    assert!(matches!(
        presentation_refused(&document),
        JoseError::InvalidContext { .. }
    ));
}

// ── type (VC DM 2.0 §4.5, §4.13) ──────────────────────────────────────────────

#[test]
fn a_credential_type_must_be_a_term_or_terms_that_include_verifiable_credential() {
    for type_ in [
        json!(null),
        json!(7),
        json!([]),
        json!(["ExampleCredential"]),
        json!("ExampleCredential"),
        json!([7, "VerifiableCredential"]),
    ] {
        let document = edited(|c| c["type"] = type_.clone());

        assert!(
            matches!(credential_refused(&document), JoseError::InvalidType { .. }),
            "{type_}"
        );
    }
    let document = edited(|c| {
        c.as_object_mut().expect("an object").remove("type");
    });
    assert!(matches!(
        credential_refused(&document),
        JoseError::InvalidType { .. }
    ));
}

#[test]
fn a_credential_type_of_a_term_or_of_terms_that_include_it_is_accepted() {
    let key = key();
    for type_ in [
        json!("VerifiableCredential"),
        json!(["VerifiableCredential", "ExampleCredential"]),
    ] {
        let document = edited(|c| c["type"] = type_.clone());

        assert!(
            sign_credential(&document, &key, KID, &[]).is_ok(),
            "{type_}"
        );
    }
}

#[test]
fn a_presentation_type_must_include_verifiable_presentation() {
    for type_ in [
        json!("VerifiableCredential"),
        json!(["VerifiableCredential"]),
        json!([]),
    ] {
        let mut document = presentation(Vec::new());
        document["type"] = type_.clone();

        assert!(
            matches!(
                presentation_refused(&document),
                JoseError::InvalidType { .. }
            ),
            "{type_}"
        );
    }
    let mut absent = presentation(Vec::new());
    absent.as_object_mut().expect("an object").remove("type");
    assert!(matches!(
        presentation_refused(&absent),
        JoseError::InvalidType { .. }
    ));

    for type_ in [
        json!("VerifiablePresentation"),
        json!(["VerifiablePresentation", "ExamplePresentation"]),
    ] {
        let mut document = presentation(Vec::new());
        document["type"] = type_.clone();
        let key = key();

        assert!(
            sign_presentation(&document, &key, KID, &[]).is_ok(),
            "{type_}"
        );
    }
}

// ── issuer (VC DM 2.0 §4.7) ───────────────────────────────────────────────────

#[test]
fn an_issuer_must_be_a_url_or_an_object_with_a_url_id() {
    for issuer in [
        json!("Example Issuer Ltd"),
        json!(7),
        json!({"name": "no id"}),
        json!({"id": "not a url"}),
    ] {
        let document = edited(|c| c["issuer"] = issuer.clone());

        assert!(
            matches!(
                credential_refused(&document),
                JoseError::InvalidIssuer { .. }
            ),
            "{issuer}"
        );
    }
    let absent = edited(|c| {
        c.as_object_mut().expect("an object").remove("issuer");
    });
    assert!(matches!(
        credential_refused(&absent),
        JoseError::InvalidIssuer { .. }
    ));
}

#[test]
fn an_issuer_that_is_a_url_or_an_object_with_a_url_id_is_accepted() {
    let key = key();
    for issuer in [
        json!("https://issuer.example"),
        json!({"id": "did:web:issuer.example", "name": "Issuer"}),
    ] {
        let document = edited(|c| c["issuer"] = issuer.clone());

        assert!(
            sign_credential(&document, &key, KID, &[]).is_ok(),
            "{issuer}"
        );
    }
}

// ── credentialSubject (VC DM 2.0 §4.8) ────────────────────────────────────────

/// The credential's subject must be present: a credential contains claims about one or
/// more subjects. Removing this requirement would let `credentialSubject` go missing.
#[test]
fn a_credential_without_a_credential_subject_is_refused() {
    let absent = edited(|c| {
        c.as_object_mut()
            .expect("an object")
            .remove("credentialSubject");
    });

    assert!(matches!(
        credential_refused(&absent),
        JoseError::InvalidCredentialSubject { .. }
    ));
}

#[test]
fn a_credential_subject_must_be_an_object_or_a_non_empty_array_of_objects() {
    for subject in [
        json!("subject"),
        json!(null),
        json!([]),
        json!([{"id": "did:example:a"}, 7]),
    ] {
        let document = edited(|c| c["credentialSubject"] = subject.clone());

        assert!(
            matches!(
                credential_refused(&document),
                JoseError::InvalidCredentialSubject { .. }
            ),
            "{subject}"
        );
    }
    let key = key();
    for subject in [
        json!({"id": "did:example:a"}),
        json!([{"id": "did:example:a"}, {"id": "did:example:b"}]),
    ] {
        let document = edited(|c| c["credentialSubject"] = subject.clone());

        assert!(
            sign_credential(&document, &key, KID, &[]).is_ok(),
            "{subject}"
        );
    }
}

// ── the holder's `kid` when there is no `iss` (VC-JOSE-COSE §4.2) ─────────────

/// When the holder is identified by a URL and `iss` is absent, the outer token's `kid`
/// must be an absolute URL. This is checked on verification too, not only at signing.
#[test]
fn a_holder_that_is_a_url_needs_an_absolute_kid_on_verification() {
    let key = key();
    let bare = outer(&key, THUMBPRINT, &presentation(Vec::new()));
    let absolute = outer(&key, KID, &presentation(Vec::new()));

    assert_eq!(
        verify_presentation(&bare, &key.public_key(), None).map(|_| ()),
        Err(JoseError::KidNotAbsolute {
            kid: Some(THUMBPRINT.to_owned())
        })
    );
    assert!(verify_presentation(&absolute, &key.public_key(), None).is_ok());
}

#[test]
fn a_holder_object_with_a_url_id_counts_as_a_url() {
    let mut document = presentation(Vec::new());
    document["holder"] = json!({"id": "did:web:holder.example"});
    let key = key();

    assert!(matches!(
        sign_presentation(&document, &key, THUMBPRINT, &[]),
        Err(JoseError::KidNotAbsolute { .. })
    ));
    assert!(matches!(
        verify_presentation(&outer(&key, THUMBPRINT, &document), &key.public_key(), None),
        Err(JoseError::KidNotAbsolute { .. })
    ));
}

#[test]
fn a_bare_kid_is_fine_with_an_iss_or_without_a_url_holder() {
    let key = key();
    let mut with_iss = presentation(Vec::new());
    with_iss["iss"] = json!("https://holder.example");
    let mut not_a_url = presentation(Vec::new());
    not_a_url["holder"] = json!("Example Holder Ltd");
    let mut no_holder = presentation(Vec::new());
    no_holder
        .as_object_mut()
        .expect("an object")
        .remove("holder");

    for document in [with_iss, not_a_url, no_holder] {
        assert!(
            verify_presentation(&outer(&key, THUMBPRINT, &document), &key.public_key(), None)
                .is_ok(),
            "{document}"
        );
    }
}
