//! The W3C VC-JOSE-COSE test suite, rows 17 to 26: the SD-JWT issuance and verification
//! inputs, under the keys the suite's mapping pairs them with.
//!
//! As in `suite_tests`, the harness's mapping is `Ok` for `success` and `Err` for
//! `failure`, and every failure row also asserts its own [`JoseError`] and that the
//! signature is valid, so the refusal is provably the check it names. The suite's
//! `--sd` paths are written here as the arrays `ClaimPath::from_json` reads, which is what
//! the harness's converter produces from the suite's dotted form.
//!
//! Every input carries an `iss` equal to its issuer, so VC-JOSE-COSE §4.2, which asks for
//! an absolute URL `kid` when `iss` is absent, does not apply to the thumbprint `kid`s
//! the verification inputs carry.

use base64::Engine as _;
use serde_json::{Value, json};

use dpp_crypto::jws::verifier::verify_jws;
use dpp_crypto::jws::{KeyAlgorithm, PublicKey};
use dpp_crypto::sd_jwt::{ClaimPath, SdJwt};

use super::super::super::VerifyOptions;
use super::super::super::{
    JoseError, sign_credential_sd_jwt, sign_presentation_sd_jwt, verify_credential_sd_jwt,
    verify_presentation_sd_jwt,
};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::{forge_raw, json, key_id, public_key, signing_key, token};

fn harness<T>(result: &Result<T, JoseError>) -> &'static str {
    if result.is_ok() { "success" } else { "failure" }
}

fn paths(paths: &Value) -> Vec<ClaimPath> {
    paths
        .as_array()
        .expect("an array")
        .iter()
        .map(|p| ClaimPath::from_json(p).expect("a path"))
        .collect()
}

/// Whether the Issuer-signed JWT of the SD-JWT in `file` verifies under `key`.
fn signature_is_valid(file: &str, key: &PublicKey) -> bool {
    let sd_jwt = SdJwt::parse(&token(file)).expect("an SD-JWT");
    verify_jws(sd_jwt.jwt(), key).expect("well-formed")
}

fn payload_of(jwt: &str) -> Value {
    let segment = jwt.split('.').nth(1).expect("a payload");
    serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(segment)
            .expect("base64url"),
    )
    .expect("JSON")
}

// ── issuance ──────────────────────────────────────────────────────────────────

#[test]
fn row_17_a_basic_credential_is_issued_with_two_claims_concealed_and_verifies_back() {
    let document = json("credential-selective.json");

    let issued = sign_credential_sd_jwt(
        &document,
        &signing_key("p384"),
        &key_id("p384"),
        &[],
        &paths(&json!([
            ["credentialSubject", "firstName"],
            ["credentialSubject", "lastName"]
        ])),
        None,
    );

    assert_eq!(harness(&issued), "success");
    let sd_jwt = issued.expect("issued");
    let signed = payload_of(sd_jwt.jwt());
    // Two digests for the claims, padded with decoys to a multiple of eight.
    assert_eq!(
        signed["credentialSubject"]["_sd"].as_array().map(Vec::len),
        Some(8)
    );
    assert_eq!(signed["credentialSubject"]["birthDate"], "1990-01-01");
    assert!(signed["credentialSubject"].get("firstName").is_none());
    assert!(signed["credentialSubject"].get("lastName").is_none());
    let verified = verify_credential_sd_jwt(
        &sd_jwt.serialise(),
        &public_key("p384"),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );
    assert_eq!(verified.expect("verifies"), document);
}

/// Row 18 conceals two members of a nested object and one element of an array, and the
/// result has the shape of the suite's own token for row 21.
#[test]
fn row_18_a_complex_credential_is_issued_with_nested_members_and_an_element_concealed() {
    let document = json("credential-nested-selective.json");

    let issued = sign_credential_sd_jwt(
        &document,
        &signing_key("p521"),
        &key_id("p521"),
        &[],
        &paths(&json!([
            ["credentialSubject", "address", "street"],
            ["credentialSubject", "address", "city"],
            ["credentialSubject", "phoneNumbers", 0]
        ])),
        None,
    );

    assert_eq!(harness(&issued), "success");
    let sd_jwt = issued.expect("issued");
    let signed = payload_of(sd_jwt.jwt());
    let subject = &signed["credentialSubject"];
    assert_eq!(subject["address"]["_sd"].as_array().map(Vec::len), Some(8));
    assert_eq!(subject["address"]["country"], "USA");
    // The first number is a placeholder among seven decoys, and the second stays in the clear.
    let phones = subject["phoneNumbers"].as_array().expect("an array");
    assert_eq!(phones.iter().filter(|p| p["..."].is_string()).count(), 8);
    assert_eq!(phones.iter().filter(|p| p["type"] == "mobile").count(), 1);
    assert_eq!(sd_jwt.disclosures().len(), 3);
    let suite_own = SdJwt::parse(&token("credential-sdjwt-nested.txt")).expect("an SD-JWT");
    assert_eq!(
        payload_of(suite_own.jwt())["credentialSubject"]
            .as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>()),
        subject
            .as_object()
            .map(|o| o.keys().cloned().collect::<Vec<_>>()),
        "the same members are in the clear as in the suite's own token"
    );
    let verified = verify_credential_sd_jwt(
        &sd_jwt.serialise(),
        &public_key("p521"),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );
    assert_eq!(verified.expect("verifies"), document);
}

/// Row 19 conceals `holder` and the one envelope. The envelope is a `vc+sd-jwt` of the
/// suite's, and it is still a `vc+sd-jwt` that verifies, after the presentation does.
#[test]
fn row_19_a_presentation_is_issued_with_its_holder_and_envelope_concealed() {
    let document = json("presentation-selective.json");

    let issued = sign_presentation_sd_jwt(
        &document,
        &signing_key("p384"),
        &key_id("p384"),
        &[],
        &paths(&json!([["holder"], ["verifiableCredential", 0]])),
        None,
    );

    assert_eq!(harness(&issued), "success");
    let sd_jwt = issued.expect("issued");
    let signed = payload_of(sd_jwt.jwt());
    assert!(signed.get("holder").is_none());
    assert!(signed["verifiableCredential"][0]["..."].is_string());
    assert_eq!(sd_jwt.disclosures().len(), 2);
    let verified = verify_presentation_sd_jwt(
        &sd_jwt.serialise(),
        &public_key("p384"),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    )
    .expect("verifies");
    assert_eq!(verified.payload(), &document);
    let envelope = &verified.envelopes()[0];
    assert_eq!(envelope.media_type, "application/vc+sd-jwt");
    assert_eq!(
        envelope.kid.as_deref(),
        Some("96qBnDpvS6x8YSCH_xmnLlXZMkDhHBr8lj99u5aog7s")
    );
    let credential = envelope
        .verify(&public_key("p384"), VerifyOptions::new(), &NOT_REQUIRED)
        .expect("verifies");
    assert_eq!(credential["credentialSubject"]["birthDate"], "1990-01-01");
}

// ── verification ──────────────────────────────────────────────────────────────

#[test]
fn row_20_a_basic_credential_verifies() {
    let result = verify_credential_sd_jwt(
        &token("credential-sdjwt-selective.txt"),
        &public_key("p384"),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );

    assert_eq!(harness(&result), "success");
    let credential = result.expect("verified");
    assert_eq!(credential["credentialSubject"]["firstName"], "Jane");
    assert_eq!(credential["credentialSubject"]["lastName"], "Doe");
    assert_eq!(credential["credentialSubject"]["birthDate"], "1990-01-01");
    assert!(credential.get("_sd_alg").is_none());
    assert!(credential["credentialSubject"].get("_sd").is_none());
}

#[test]
fn row_21_a_complex_credential_verifies_with_its_nested_members_and_element() {
    let result = verify_credential_sd_jwt(
        &token("credential-sdjwt-nested.txt"),
        &public_key("p521"),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );

    assert_eq!(harness(&result), "success");
    let subject = &result.expect("verified")["credentialSubject"];
    assert_eq!(subject["address"]["street"], "123 Main St");
    assert_eq!(subject["address"]["city"], "Anytown");
    assert_eq!(subject["phoneNumbers"][0]["type"], "work");
    assert_eq!(subject["phoneNumbers"][1]["type"], "mobile");
}

/// The suite's presentation conceals `holder` and `type`, and discloses both. Its one
/// envelope is a `vc+jwt` signed by an Ed25519 key the suite names, and is not verified by
/// verifying the presentation. Under that key it verifies.
#[test]
fn row_22_a_presentation_verifies_and_lists_its_envelope_unverified() {
    let result = verify_presentation_sd_jwt(
        &token("presentation-sdjwt-selective.txt"),
        &public_key("p384"),
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );

    assert_eq!(harness(&result), "success");
    let presentation = result.expect("verified");
    assert_eq!(presentation.payload()["type"], "VerifiablePresentation");
    assert_eq!(
        presentation.payload()["holder"],
        "https://example.issuer/vc-jose-cose"
    );
    let listed: Vec<_> = presentation
        .envelopes()
        .iter()
        .map(|e| (e.index, e.media_type.as_str(), e.kid.as_deref()))
        .collect();
    assert_eq!(
        listed,
        [(
            0,
            "application/vc+jwt",
            Some("2aOHaQxsWVT4mLsUkdtlCnUV-au0BBR-5yPYaPaCc5k")
        )]
    );
    let credential = presentation.envelopes()[0]
        .verify(&public_key("ed25519"), VerifyOptions::new(), &NOT_REQUIRED)
        .expect("the vc+jwt verifies under Ed25519");
    assert_eq!(credential["issuer"], "https://example.issuer/vc-jose-cose");
}

#[test]
fn row_23_a_bad_signature_is_refused() {
    let key = public_key("p384");
    let result = verify_credential_sd_jwt(
        &token("credential-sdjwt-bad-signature.txt"),
        &key,
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::SignatureDoesNotVerify
    );
    // The token is row 20's, with its signature changed: header and payload agree.
    let good = SdJwt::parse(&token("credential-sdjwt-selective.txt")).expect("an SD-JWT");
    let bad = SdJwt::parse(&token("credential-sdjwt-bad-signature.txt")).expect("an SD-JWT");
    assert_eq!(
        good.jwt().rsplit_once('.').map(|(signed, _)| signed),
        bad.jwt().rsplit_once('.').map(|(signed, _)| signed)
    );
    assert_eq!(good.disclosures(), bad.disclosures());
}

#[test]
fn row_24_a_credential_with_a_bad_media_type_is_refused_for_its_typ() {
    let key = public_key("ed25519");
    assert!(signature_is_valid(
        "credential-sdjwt-bad-media-type.txt",
        &key
    ));

    let result = verify_credential_sd_jwt(
        &token("credential-sdjwt-bad-media-type.txt"),
        &key,
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );

    assert_eq!(harness(&result), "failure");
    // `cty` is bad too, and `typ` is read first.
    assert_eq!(
        result.expect_err("refused"),
        JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"bad+typ\"".to_owned()),
            expected: "vc+sd-jwt",
        }
    );
}

#[test]
fn row_25_a_presentation_with_a_bad_media_type_is_refused_for_its_typ() {
    let key = public_key("ed25519");
    assert!(signature_is_valid(
        "presentation-sdjwt-bad-media-type.txt",
        &key
    ));

    let result = verify_presentation_sd_jwt(
        &token("presentation-sdjwt-bad-media-type.txt"),
        &key,
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"bad+typ\"".to_owned()),
            expected: "vp+sd-jwt",
        }
    );
}

/// Row 26's token has a bad `typ` and `cty`, and its one envelope is bad as well, so the
/// refusal the library gives is for `typ`, which is read first. The same token with its
/// header corrected and signed again, under the P-521 key the suite names, is refused for
/// the envelope, so the row is refused for two defects and not for one that hides the other.
#[test]
fn row_26_a_presentation_with_an_invalid_credential_is_refused() {
    let key = public_key("p521");
    assert!(signature_is_valid(
        "presentation-sdjwt-bad-credential.txt",
        &key
    ));

    let result = verify_presentation_sd_jwt(
        &token("presentation-sdjwt-bad-credential.txt"),
        &key,
        VerifyOptions::new(),
        &NOT_REQUIRED,
    );

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"bad+typ\"".to_owned()),
            expected: "vp+sd-jwt",
        }
    );

    let original =
        SdJwt::parse(&token("presentation-sdjwt-bad-credential.txt")).expect("an SD-JWT");
    let mut segments = original.jwt().split('.');
    let header: Value = serde_json::from_slice(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(segments.next().expect("a header"))
            .expect("base64url"),
    )
    .expect("JSON");
    let corrected = {
        let mut header = header;
        header["typ"] = json!("vp+sd-jwt");
        header["cty"] = json!("vp");
        header
    };
    let resigned = SdJwt::new(
        forge_raw(
            &signing_key("p521"),
            &corrected,
            segments.next().expect("a payload"),
        ),
        original.disclosures().to_vec(),
    );
    assert_eq!(
        verify_presentation_sd_jwt(
            &resigned.serialise(),
            &key,
            VerifyOptions::new(),
            &NOT_REQUIRED
        )
        .map(|_| ()),
        Err(JoseError::MalformedEnvelope {
            index: 0,
            reason: "id is not a data: URL".to_owned(),
        })
    );
}

/// The thumbprint `kid`s and the P-521 alg of these inputs are what the suite's keys have.
#[test]
fn the_suites_keys_are_the_ones_the_rows_name() {
    for (file, alg) in [
        ("credential-sdjwt-selective.txt", KeyAlgorithm::EcdsaP384),
        ("credential-sdjwt-nested.txt", KeyAlgorithm::EcdsaP521),
        ("presentation-sdjwt-selective.txt", KeyAlgorithm::EcdsaP384),
    ] {
        let key = match alg {
            KeyAlgorithm::EcdsaP384 => public_key("p384"),
            _ => public_key("p521"),
        };
        assert_eq!(key.algorithm(), alg, "{file}");
        assert!(signature_is_valid(file, &key), "{file}");
    }
}
