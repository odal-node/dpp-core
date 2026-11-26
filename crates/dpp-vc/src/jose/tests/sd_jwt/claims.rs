//! Verifying an SD-JWT, past the Issuer-signed JWT and the Disclosures: the processed
//! payload and the checks it is held to, the claims RFC 9901 clause 9.7 keeps out of
//! Disclosures, time, what signing makes and verification returns, and presentations.

use serde_json::{Value, json};

use dpp_crypto::jws::KeyAlgorithm;
use dpp_crypto::keystore::KeyEntry;
use dpp_crypto::sd_jwt::{ClaimPath, SdJwt, build_payload, conceal_elements};

use super::super::super::VerifyOptions;
use super::super::super::{
    JoseError, sign_credential_sd_jwt, sign_presentation_sd_jwt, verify_credential_sd_jwt,
    verify_presentation_sd_jwt,
};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::at;
use crate::test_support::{
    KID, credential, forge_sd, forge_sd_concealing, generated, presentation, sd_credential_header,
    sd_presentation_header,
};

const NOW: i64 = 1_800_000_000;

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

/// A change to a document.
type Change = Box<dyn Fn(&mut Value)>;
type Refusal = fn(&JoseError) -> bool;

fn path(segments: &Value) -> ClaimPath {
    ClaimPath::from_json(segments).expect("a path")
}

fn concealing(key: &KeyEntry, conceal: &[&str]) -> String {
    forge_sd_concealing(key, &sd_credential_header(key), &credential(), conceal)
}

// ── the processed payload ─────────────────────────────────────────────────────

/// The suite's own presentation conceals `type`, and discloses it. The checks read the
/// processed payload, so a concealed claim that is disclosed is read where it belongs.
#[test]
fn a_claim_concealed_and_disclosed_is_read_as_if_it_had_been_in_the_clear() {
    let key = key();

    let token = concealing(&key, &["type", "issuer", "credentialSubject", "@context"]);

    assert_eq!(verify(&token, &key), Ok(credential()));
}

/// A claim the holder did not disclose is not there, and the checks refuse the credential
/// for lacking it.
#[test]
fn a_required_claim_that_was_not_disclosed_is_missing_to_the_checks() {
    let key = key();
    let cases: [(&str, Refusal); 4] = [
        ("type", |e| matches!(e, JoseError::InvalidType { .. })),
        ("issuer", |e| matches!(e, JoseError::InvalidIssuer { .. })),
        ("credentialSubject", |e| {
            matches!(e, JoseError::InvalidCredentialSubject { .. })
        }),
        ("@context", |e| {
            matches!(e, JoseError::InvalidContext { .. })
        }),
    ];
    for (name, expected) in cases {
        let full = SdJwt::parse(&concealing(&key, &[name])).expect("parses");
        let withheld = SdJwt::new(full.jwt().to_owned(), vec![]).serialise();

        let refused = verify(&withheld, &key).expect_err("refused");

        assert!(expected(&refused), "{name}: {refused:?}");
    }
}

#[test]
fn what_verify_credential_refuses_in_the_claims_is_refused_here() {
    let key = key();
    let cases: [(Change, JoseError); 4] = [
        (
            Box::new(|d| d["vp"] = json!({})),
            JoseError::ReservedClaim { name: "vp" },
        ),
        (
            Box::new(|d| d["iss"] = json!("https://elsewhere.example")),
            JoseError::IssuerMismatch {
                iss: "https://elsewhere.example".to_owned(),
                issuer: Some("https://issuer.example".to_owned()),
            },
        ),
        (
            Box::new(|d| d["issuer"] = json!("not a url")),
            JoseError::InvalidIssuer {
                reason: "is neither a URL nor an object with a URL id".to_owned(),
            },
        ),
        (
            Box::new(|d| d["@context"] = json!(["https://example.org/other"])),
            JoseError::InvalidContext {
                reason: "does not begin with https://www.w3.org/ns/credentials/v2".to_owned(),
            },
        ),
    ];
    for (change, expected) in cases {
        let mut document = credential();
        change(&mut document);

        let token = forge_sd(&key, &sd_credential_header(&key), &document, &[]);

        assert_eq!(verify(&token, &key), Err(expected));
    }
}

/// A `vc` claim in a Disclosure is a `vc` claim once the Disclosures are applied.
#[test]
fn a_reserved_claim_hidden_in_a_disclosure_is_refused_when_it_is_disclosed() {
    let key = key();
    let mut document = credential();
    document["vc"] = json!({});

    let token = forge_sd_concealing(&key, &sd_credential_header(&key), &document, &["vc"]);

    assert_eq!(
        verify(&token, &key),
        Err(JoseError::ReservedClaim { name: "vc" })
    );
}

/// Section 4.2 opens "When using controlled identifier documents with this specification",
/// and a verifier here is given its key, so a `kid` that is not an absolute URL verifies, as it
/// does for a `vc+jwt`. The signer refuses it: see `sd_jwt_refusal_tests`.
#[test]
fn without_an_iss_a_kid_that_is_not_an_absolute_url_verifies() {
    let key = key();
    let mut header = sd_credential_header(&key);
    header["kid"] = json!("96qBnDpvS6x8YSCH_xmnLlXZMkDhHBr8lj99u5aog7s");
    let without_iss = forge_sd(&key, &header, &credential(), &[]);
    let mut with_iss = credential();
    with_iss["iss"] = json!("https://issuer.example");
    let with_iss = forge_sd(&key, &header, &with_iss, &[]);

    assert!(verify(&without_iss, &key).is_ok());
    assert!(verify(&with_iss, &key).is_ok());
}

// ── the claims RFC 9901 clause 9.7 keeps out of Disclosures ───────────────────

/// A token that conceals `exp`, `nbf` or `aud`: our signer refuses to make one (RFC 9901
/// clause 9.7), so it is built with the lower-level builder.
fn disclosed(key: &KeyEntry, name: &str, value: Value) -> String {
    let mut document = credential();
    document[name] = value;
    forge_sd_concealing(key, &sd_credential_header(key), &document, &[name])
}

/// An SD-JWT over `document` in which the entry of its `aud` at `position` is a Disclosure.
fn forge_sd_concealing_aud_entry(key: &KeyEntry, document: &Value, position: usize) -> String {
    let mut document = document.as_object().expect("an object").clone();
    let entries = document["aud"].as_array().expect("an array").clone();
    let (kept, disclosures) = conceal_elements(&entries, |at, _| at == position);
    document.insert("aud".to_owned(), Value::Array(kept));
    let payload = build_payload(document, true);
    forge_sd(key, &sd_credential_header(key), &payload, &disclosures)
}

/// §7.1 step 6 has the verifier check the claims in the processed payload, so a claim that
/// arrived in a Disclosure is checked as one in the clear: an `exp` in the past is refused.
#[test]
fn a_disclosed_exp_in_the_past_is_refused_as_expired() {
    let key = key();
    let token = disclosed(&key, "exp", json!(NOW - 1));

    let refused = verify_credential_sd_jwt(
        &token,
        &key.public_key(),
        VerifyOptions::new().at(NOW),
        &NOT_REQUIRED,
    );

    assert_eq!(refused, Err(JoseError::Expired));
}

#[test]
fn a_disclosed_future_exp_is_accepted() {
    let key = key();
    let token = disclosed(&key, "exp", json!(NOW + 1));

    let verified = verify_credential_sd_jwt(
        &token,
        &key.public_key(),
        VerifyOptions::new().at(NOW),
        &NOT_REQUIRED,
    )
    .expect("verifies");

    assert_eq!(verified["exp"], json!(NOW + 1));
}

#[test]
fn a_disclosed_nbf_in_the_future_is_refused_as_not_yet_valid() {
    let key = key();
    let token = disclosed(&key, "nbf", json!(NOW + 1));

    let refused = verify_credential_sd_jwt(
        &token,
        &key.public_key(),
        VerifyOptions::new().at(NOW),
        &NOT_REQUIRED,
    );

    assert_eq!(refused, Err(JoseError::NotYetValid));
}

/// RFC 9901 §9.7: "issuers MAY allow individual entries in the array to be selectively
/// disclosable". The entry is part of the processed payload's `aud`, and the audience is
/// compared with that.
#[test]
fn a_disclosed_aud_entry_naming_the_audience_is_accepted() {
    let key = key();
    let mut document = credential();
    document["aud"] = json!(["https://other.example", "https://verifier.example"]);
    let sd_jwt = forge_sd_concealing_aud_entry(&key, &document, 1);

    let verified = verify_credential_sd_jwt(
        &sd_jwt,
        &key.public_key(),
        VerifyOptions::new().for_audience("https://verifier.example"),
        &NOT_REQUIRED,
    );

    assert!(verified.is_ok(), "{verified:?}");
}

/// A whole `aud` that arrived in a Disclosure is read as one in the clear.
#[test]
fn a_disclosed_aud_is_compared_with_the_audience() {
    let key = key();
    let token = disclosed(&key, "aud", json!("https://verifier.example"));

    let named = verify_credential_sd_jwt(
        &token,
        &key.public_key(),
        VerifyOptions::new().for_audience("https://verifier.example"),
        &NOT_REQUIRED,
    );
    let other = verify_credential_sd_jwt(
        &token,
        &key.public_key(),
        VerifyOptions::new().for_audience("https://elsewhere.example"),
        &NOT_REQUIRED,
    );

    assert!(named.is_ok());
    assert_eq!(
        other,
        Err(JoseError::AudienceMismatch {
            audience: Some("https://elsewhere.example".to_owned())
        })
    );
}

/// What cannot be seen is not refused: a Disclosure that was withheld leaves no trace but
/// its digest. A verifier cannot tell it from a claim that was never there, which is why
/// RFC 9901 clause 9.7 puts the duty on the Issuer, and our signer refuses to conceal one.
#[test]
fn a_withheld_exp_is_not_seen() {
    let key = key();
    let mut document = credential();
    document["exp"] = json!(1_000_000_000);
    let full = SdJwt::parse(&forge_sd_concealing(
        &key,
        &sd_credential_header(&key),
        &document,
        &["exp"],
    ))
    .expect("parses");
    let withheld = SdJwt::new(full.jwt().to_owned(), vec![]).serialise();

    assert!(
        verify_credential_sd_jwt(
            &withheld,
            &key.public_key(),
            VerifyOptions::new().at(NOW),
            &NOT_REQUIRED
        )
        .is_ok()
    );
}

// ── time ──────────────────────────────────────────────────────────────────────

#[test]
fn time_is_held_to_the_now_given() {
    let key = key();
    let mut document = credential();
    document["exp"] = json!(NOW);
    document["nbf"] = json!(NOW - 100);
    let token = forge_sd(&key, &sd_credential_header(&key), &document, &[]);
    let at = |now| {
        verify_credential_sd_jwt(&token, &key.public_key(), at(now), &NOT_REQUIRED).map(|_| ())
    };

    assert_eq!(at(Some(NOW - 1)), Ok(()));
    assert_eq!(at(Some(NOW)), Err(JoseError::Expired));
    assert_eq!(at(Some(NOW - 101)), Err(JoseError::NotYetValid));
    assert_eq!(at(None), Ok(()));
    document["exp"] = json!("soon");
    let malformed = forge_sd(&key, &sd_credential_header(&key), &document, &[]);
    assert_eq!(
        verify_credential_sd_jwt(
            &malformed,
            &key.public_key(),
            VerifyOptions::new().at(NOW),
            &NOT_REQUIRED
        )
        .map(|_| ()),
        Err(JoseError::MalformedTemporalClaim { claim: "exp" })
    );
}

// ── what signing makes, verification returns ──────────────────────────────────

#[test]
fn only_what_the_holder_discloses_comes_back() {
    let key = key();
    let mut document = credential();
    document["credentialSubject"]["email"] = json!("a@example.org");
    let sd_jwt = sign_credential_sd_jwt(
        &document,
        &key,
        KID,
        &[],
        &[
            path(&json!(["credentialSubject", "name"])),
            path(&json!(["credentialSubject", "email"])),
        ],
        None,
    )
    .expect("signs");
    let email = sd_jwt.digests_for_claim("email");
    let reveal: Vec<&str> = email.iter().map(String::as_str).collect();

    let presented = sd_jwt.present(&reveal).expect("presents");
    let verified = verify(&presented.serialise(), &key).expect("verifies");

    assert_eq!(verified["credentialSubject"]["email"], "a@example.org");
    assert!(verified["credentialSubject"].get("name").is_none());
    assert_eq!(
        verify(&sd_jwt.serialise(), &key).expect("verifies"),
        document
    );
}

// ── presentations ─────────────────────────────────────────────────────────────

fn verify_vp(
    token: &str,
    key: &KeyEntry,
) -> Result<super::super::super::VerifiedPresentation, JoseError> {
    verify_presentation_sd_jwt(
        token,
        &key.public_key(),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    )
}

#[test]
fn a_presentation_is_verified_as_its_own_token_and_lists_its_envelopes() {
    let key = key();
    let issuer = generated(KeyAlgorithm::Ed25519);
    let inner = sign_credential_sd_jwt(&credential(), &issuer, KID, &[], &[], None).expect("signs");
    let document = presentation(vec![crate::test_support::sd_envelope(&inner.serialise())]);

    let sd_jwt = sign_presentation_sd_jwt(&document, &key, KID, &[], &[], None).expect("signs");
    let verified = verify_vp(&sd_jwt.serialise(), &key).expect("verifies");

    assert_eq!(verified.payload(), &document);
    assert_eq!(verified.envelopes().len(), 1);
    assert_eq!(verified.envelopes()[0].media_type, "application/vc+sd-jwt");
    assert_eq!(verified.envelopes()[0].kid.as_deref(), Some(KID));
}

#[test]
fn a_presentation_is_held_to_vp_sd_jwt_and_to_the_presentation_rules() {
    let key = key();
    let forged = |header: &Value, payload: &Value| forge_sd(&key, header, payload, &[]);

    assert_eq!(
        verify_vp(
            &forged(&sd_credential_header(&key), &presentation(vec![])),
            &key
        )
        .map(|_| ()),
        Err(JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"vc+sd-jwt\"".to_owned()),
            expected: "vp+sd-jwt",
        })
    );
    let mut not_a_presentation = presentation(vec![]);
    not_a_presentation["type"] = json!(["VerifiableCredential"]);
    assert!(matches!(
        verify_vp(
            &forged(&sd_presentation_header(&key), &not_a_presentation),
            &key
        ),
        Err(JoseError::InvalidType { .. })
    ));
    let mut bad_holder = presentation(vec![]);
    bad_holder["holder"] = json!("not a url");
    assert!(matches!(
        verify_vp(&forged(&sd_presentation_header(&key), &bad_holder), &key),
        Err(JoseError::InvalidHolder { .. })
    ));
    let header = {
        let mut h = sd_presentation_header(&key);
        h["kid"] = json!("thumbprint");
        h
    };
    assert!(verify_vp(&forged(&header, &presentation(vec![])), &key).is_ok());
}

/// An envelope that was concealed and not disclosed is not in the presentation; one that
/// was disclosed is checked like any other.
#[test]
fn a_concealed_envelope_is_checked_when_it_is_disclosed_and_absent_when_it_is_not() {
    let key = key();
    let issuer = generated(KeyAlgorithm::Ed25519);
    let inner = sign_credential_sd_jwt(&credential(), &issuer, KID, &[], &[], None).expect("signs");
    let document = presentation(vec![crate::test_support::sd_envelope(&inner.serialise())]);
    let sd_jwt = sign_presentation_sd_jwt(
        &document,
        &key,
        KID,
        &[],
        &[
            path(&json!(["holder"])),
            path(&json!(["verifiableCredential", 0])),
        ],
        None,
    )
    .expect("signs");

    let disclosed = verify_vp(&sd_jwt.serialise(), &key).expect("verifies");
    let withheld = verify_vp(
        &SdJwt::new(sd_jwt.jwt().to_owned(), vec![]).serialise(),
        &key,
    )
    .expect("verifies");

    assert_eq!(disclosed.envelopes().len(), 1);
    assert_eq!(withheld.envelopes().len(), 0);
    assert!(withheld.payload().get("holder").is_none());
}
