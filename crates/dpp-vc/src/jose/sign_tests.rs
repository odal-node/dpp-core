//! Signing: what goes into the header and payload, and what is refused before
//! anything is signed.
//!
//! The vocabulary rule has its own file, `vocabulary_tests`.

use base64::Engine as _;
use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::jws::canonical::canonicalize;

use super::test_fixtures::{KID, credential, generated, presentation};
use super::{JoseError, sign_credential, sign_presentation, verify_credential};

fn segment(token: &str, index: usize) -> Vec<u8> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(token.split('.').nth(index).expect("a segment"))
        .expect("base64url")
}

#[test]
fn the_header_names_the_key_the_kid_and_the_type() {
    let key = generated(KeyAlgorithm::EcdsaP256);

    let token = sign_credential(&credential(), &key, KID, &[]).expect("signs");

    let header: Value = serde_json::from_slice(&segment(&token, 0)).expect("JSON");
    assert_eq!(
        header,
        json!({"alg": "ES256", "kid": KID, "typ": "vc+jwt", "cty": "vc"})
    );
}

/// The payload is the document, canonicalised, with nothing added: no `iat`, no
/// `iss`, no `vc` claim. Signing the same document twice gives the same token.
#[test]
fn the_payload_is_the_document_in_canonical_form_with_no_claim_added() {
    let key = generated(KeyAlgorithm::Ed25519);
    let document = credential();

    let token = sign_credential(&document, &key, KID, &[]).expect("signs");

    assert_eq!(
        segment(&token, 1),
        canonicalize(&document).expect("canonical")
    );
    assert_eq!(
        sign_credential(&document, &key, KID, &[]).expect("signs"),
        token
    );
}

#[test]
fn a_presentation_is_typed_vp_jwt() {
    let key = generated(KeyAlgorithm::Ed25519);

    let token = sign_presentation(&presentation(Vec::new()), &key, KID, &[]).expect("signs");

    let header: Value = serde_json::from_slice(&segment(&token, 0)).expect("JSON");
    assert_eq!(header["typ"], "vp+jwt");
    assert_eq!(header["cty"], "vp");
}

#[test]
fn something_that_is_not_an_object_is_not_signed() {
    let key = generated(KeyAlgorithm::Ed25519);
    for document in [json!([]), json!("text"), json!(null), json!(7)] {
        assert_eq!(
            sign_credential(&document, &key, KID, &[]),
            Err(JoseError::NotAnObject { what: "credential" })
        );
        assert_eq!(
            sign_presentation(&document, &key, KID, &[]),
            Err(JoseError::NotAnObject {
                what: "presentation"
            })
        );
    }
}

#[test]
fn a_document_without_a_usable_context_is_not_signed() {
    let key = generated(KeyAlgorithm::Ed25519);
    for context in [None, Some(json!(7)), Some(json!(null)), Some(json!({}))] {
        let mut document = credential();
        let object = document.as_object_mut().expect("an object");
        match &context {
            Some(context) => object.insert("@context".to_owned(), context.clone()),
            None => object.remove("@context"),
        };

        let refused = sign_credential(&document, &key, KID, &[]).expect_err("refused");

        assert!(
            matches!(refused, JoseError::MalformedPayload { .. }),
            "{context:?}: {refused:?}"
        );
    }
    let mut document = presentation(Vec::new());
    document
        .as_object_mut()
        .expect("an object")
        .remove("@context");
    assert!(matches!(
        sign_presentation(&document, &key, KID, &[]),
        Err(JoseError::MalformedPayload { .. })
    ));
}

// ── §4.2: the `kid` when there is no `iss` ────────────────────────────────────

const THUMBPRINT: &str = "73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU";

#[test]
fn a_bare_kid_is_refused_when_there_is_no_iss_and_the_issuer_is_a_url() {
    let key = generated(KeyAlgorithm::Ed25519);

    assert_eq!(
        sign_credential(&credential(), &key, THUMBPRINT, &[]),
        Err(JoseError::KidNotAbsolute {
            kid: Some(THUMBPRINT.to_owned())
        })
    );
}

#[test]
fn an_issuer_that_is_an_object_with_a_url_id_counts_as_a_url() {
    let key = generated(KeyAlgorithm::Ed25519);
    let mut document = credential();
    document["issuer"] = json!({"id": "did:web:issuer.example", "name": "Issuer"});

    assert!(matches!(
        sign_credential(&document, &key, THUMBPRINT, &[]),
        Err(JoseError::KidNotAbsolute { .. })
    ));
    assert!(sign_credential(&document, &key, "did:web:issuer.example#key-1", &[]).is_ok());
}

#[test]
fn the_other_absolute_forms_pass() {
    let key = generated(KeyAlgorithm::Ed25519);
    for kid in [
        KID,
        "did:web:issuer.example#key-1",
        "urn:ietf:params:oauth:jwk-thumbprint:sha-256:73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU",
    ] {
        assert!(
            sign_credential(&credential(), &key, kid, &[]).is_ok(),
            "{kid}"
        );
    }
    for kid in [
        "",
        "key-1",
        "#key-1",
        "://x",
        "1http://x",
        "http: //x",
        "http:",
    ] {
        assert!(
            matches!(
                sign_credential(&credential(), &key, kid, &[]),
                Err(JoseError::KidNotAbsolute { .. })
            ),
            "{kid:?}"
        );
    }
}

#[test]
fn a_bare_kid_is_fine_when_the_credential_has_an_iss() {
    let key = generated(KeyAlgorithm::Ed25519);
    let mut document = credential();
    document["iss"] = json!("https://issuer.example");

    let token = sign_credential(&document, &key, THUMBPRINT, &[]).expect("signs");

    assert!(verify_credential(&token, &key.public_key(), None).is_ok());
}

#[test]
fn a_bare_kid_is_fine_when_the_issuer_is_not_a_url() {
    let key = generated(KeyAlgorithm::Ed25519);
    let mut document = credential();
    document["issuer"] = json!("Example Issuer Ltd");

    assert!(sign_credential(&document, &key, THUMBPRINT, &[]).is_ok());
}

#[test]
fn the_kid_rule_is_about_a_credentials_issuer_and_not_a_presentations() {
    let key = generated(KeyAlgorithm::Ed25519);

    assert!(sign_presentation(&presentation(Vec::new()), &key, THUMBPRINT, &[]).is_ok());
}
