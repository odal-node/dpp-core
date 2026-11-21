//! Signing: what goes into the header and payload, and what is refused before
//! anything is signed.
//!
//! The vocabulary rule has its own file, `vocabulary_tests`.

use base64::Engine as _;
use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::jws::canonical::canonicalize;

use super::VerifyOptions;
use super::{
    JoseError, sign_credential, sign_credential_sd_jwt, sign_presentation,
    sign_presentation_sd_jwt, verify_credential, verify_presentation,
};
use crate::test_support::{
    KID, credential, credential_header, forge, generated, presentation, presentation_header,
};

/// The time a verifier checks against: the signer compares no time, so only verification
/// under this `now` refuses an `exp` or `nbf` that is not a number.
const NOW: i64 = 1_800_000_000;

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
            matches!(refused, JoseError::InvalidContext { .. }),
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
        Err(JoseError::InvalidContext { .. })
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

    assert!(verify_credential(&token, &key.public_key(), VerifyOptions::new()).is_ok());
}

/// An issuer that is not a URL is refused for the issuer itself (VC DM 2.0 §4.7), so
/// the bare kid is never what fails, and the kid rule has no URL issuer to apply to.
#[test]
fn a_credential_with_a_non_url_issuer_is_refused_as_an_invalid_issuer() {
    let key = generated(KeyAlgorithm::Ed25519);
    let mut document = credential();
    document["issuer"] = json!("Example Issuer Ltd");

    assert!(matches!(
        sign_credential(&document, &key, THUMBPRINT, &[]),
        Err(JoseError::InvalidIssuer { .. })
    ));
}

/// The §4.2 rule for a presentation: with no `iss` and a holder that is a URL, the
/// `kid` must be absolute, as it is for a credential's issuer.
#[test]
fn the_kid_rule_applies_to_a_presentations_holder_as_to_a_credentials_issuer() {
    let key = generated(KeyAlgorithm::Ed25519);

    assert_eq!(
        sign_presentation(&presentation(Vec::new()), &key, THUMBPRINT, &[]),
        Err(JoseError::KidNotAbsolute {
            kid: Some(THUMBPRINT.to_owned())
        })
    );
    assert!(sign_presentation(&presentation(Vec::new()), &key, KID, &[]).is_ok());

    // A holder that is not a URL is refused for the holder itself, before the kid rule is read.
    let mut not_a_url = presentation(Vec::new());
    not_a_url["holder"] = json!("Example Holder Ltd");
    assert_eq!(
        sign_presentation(&not_a_url, &key, THUMBPRINT, &[]),
        Err(JoseError::InvalidHolder {
            reason: "is neither a URL nor an object with a URL id".to_owned(),
        })
    );

    let mut with_iss = presentation(Vec::new());
    with_iss["iss"] = json!("https://holder.example");
    assert!(sign_presentation(&with_iss, &key, THUMBPRINT, &[]).is_ok());
}

// ── what verification refuses, signing refuses too ────────────────────────────

/// An `iss` that is not the issuer, or its `id`, is refused by verification as
/// `IssuerMismatch`, and signing refuses the same document under the same error.
#[test]
fn an_iss_that_is_not_the_issuer_is_refused_at_signing() {
    let key = generated(KeyAlgorithm::Ed25519);
    let mut document = credential();
    document["iss"] = json!("https://attacker.example");
    let refused = JoseError::IssuerMismatch {
        iss: "https://attacker.example".to_owned(),
        issuer: Some("https://issuer.example".to_owned()),
    };

    assert_eq!(
        sign_credential(&document, &key, KID, &[]),
        Err(refused.clone())
    );
    let token = forge(&key, &credential_header(&key), &document);
    assert_eq!(
        verify_credential(&token, &key.public_key(), VerifyOptions::new()),
        Err(refused)
    );

    document["iss"] = json!("https://issuer.example");
    assert!(sign_credential(&document, &key, KID, &[]).is_ok());
    document["issuer"] = json!({"id": "https://issuer.example"});
    assert!(sign_credential(&document, &key, KID, &[]).is_ok());
}

/// An `iss` that is not a string is refused by verification as a malformed payload, and
/// by signing too.
#[test]
fn an_iss_that_is_not_a_string_is_refused_at_signing() {
    let key = generated(KeyAlgorithm::Ed25519);
    let mut document = credential();
    document["iss"] = json!(["https://issuer.example"]);

    assert!(matches!(
        sign_credential(&document, &key, KID, &[]),
        Err(JoseError::MalformedPayload { .. })
    ));
    let token = forge(&key, &credential_header(&key), &document);
    assert!(matches!(
        verify_credential(&token, &key.public_key(), VerifyOptions::new().at(NOW)),
        Err(JoseError::MalformedPayload { .. })
    ));
}

/// An `exp` or `nbf` that is not a JSON number is refused by verification under a `now`,
/// and signing refuses it too. A number signs, whatever it says about time.
#[test]
fn an_exp_or_nbf_that_is_not_a_number_is_refused_at_signing() {
    let key = generated(KeyAlgorithm::Ed25519);
    for claim in ["exp", "nbf"] {
        for value in [
            json!("1900000000"),
            json!(null),
            json!(true),
            json!([1_900_000_000]),
            json!({}),
        ] {
            let mut document = credential();
            document[claim] = value.clone();
            let refused = JoseError::MalformedTemporalClaim { claim };

            assert_eq!(
                sign_credential(&document, &key, KID, &[]),
                Err(refused.clone()),
                "{claim}: {value}"
            );
            let token = forge(&key, &credential_header(&key), &document);
            assert_eq!(
                verify_credential(&token, &key.public_key(), VerifyOptions::new().at(NOW)),
                Err(refused),
                "{claim}: {value}"
            );
        }

        // A number signs even when it is in the past: the signer compares no time.
        let mut document = credential();
        document[claim] = json!(1);
        assert!(
            sign_credential(&document, &key, KID, &[]).is_ok(),
            "{claim}"
        );
    }
}

#[test]
fn an_exp_that_is_not_a_number_is_refused_at_presentation_signing() {
    let key = generated(KeyAlgorithm::Ed25519);
    let mut document = presentation(Vec::new());
    document["exp"] = json!("tomorrow");
    let refused = JoseError::MalformedTemporalClaim { claim: "exp" };

    assert_eq!(
        sign_presentation(&document, &key, KID, &[]),
        Err(refused.clone())
    );
    let token = forge(&key, &presentation_header(&key), &document);
    assert_eq!(
        verify_presentation(&token, &key.public_key(), VerifyOptions::new().at(NOW)).map(|_| ()),
        Err(refused)
    );
}

/// RFC 7519 §4.1.6: "Its value MUST be a number containing a NumericDate value." So signing
/// refuses an `iat` that is not one, exactly as it refuses `exp` and `nbf`, in the `vc+jwt`
/// and SD-JWT forms of a credential and of a presentation. Verification does not read `iat`
/// (see `iat_in_any_form_is_accepted`), because the suite's credentials carry a date string.
#[test]
fn an_iat_that_is_not_a_number_is_refused_at_signing() {
    let key = generated(KeyAlgorithm::Ed25519);
    let refused = JoseError::MalformedTemporalClaim { claim: "iat" };
    for value in [
        json!("2025-01-01T00:00:00Z"),
        json!(null),
        json!(true),
        json!([1_700_000_000]),
        json!({}),
    ] {
        let mut vc = credential();
        vc["iat"] = value.clone();
        let mut vp = presentation(Vec::new());
        vp["iat"] = value.clone();

        assert_eq!(
            sign_credential(&vc, &key, KID, &[]),
            Err(refused.clone()),
            "{value}"
        );
        assert_eq!(
            sign_presentation(&vp, &key, KID, &[]),
            Err(refused.clone()),
            "{value}"
        );
        assert_eq!(
            sign_credential_sd_jwt(&vc, &key, KID, &[], &[], None).map(|_| ()),
            Err(refused.clone()),
            "{value}"
        );
        assert_eq!(
            sign_presentation_sd_jwt(&vp, &key, KID, &[], &[], None).map(|_| ()),
            Err(refused.clone()),
            "{value}"
        );
    }
}

#[test]
fn an_iat_that_is_a_number_is_signed() {
    let key = generated(KeyAlgorithm::Ed25519);
    for value in [json!(1_700_000_000), json!(1_700_000_000.5), json!(0)] {
        let mut vc = credential();
        vc["iat"] = value.clone();
        let mut vp = presentation(Vec::new());
        vp["iat"] = value.clone();

        assert!(sign_credential(&vc, &key, KID, &[]).is_ok(), "{value}");
        assert!(sign_presentation(&vp, &key, KID, &[]).is_ok(), "{value}");
        assert!(
            sign_credential_sd_jwt(&vc, &key, KID, &[], &[], None).is_ok(),
            "{value}"
        );
        assert!(
            sign_presentation_sd_jwt(&vp, &key, KID, &[], &[], None).is_ok(),
            "{value}"
        );
    }
}
