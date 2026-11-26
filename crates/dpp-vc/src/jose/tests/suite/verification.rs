//! The W3C VC-JOSE-COSE test suite, rows 6 to 16: every verification input, under
//! the key the suite's mapping pairs it with, gets the verdict the suite expects.
//!
//! The harness's mapping is `Ok` for `success` and `Err` for `failure`, and
//! [`harness`] is that mapping. Every row also asserts the specific
//! [`JoseError`], and for the rows that are refused for a header or a claim, that
//! the signature is valid, so the refusal is provably the check it names and not a
//! signature that never verified. Rows 1 to 5, which issue, are in
//! `suite_issue_tests`.
//!
//! The suite has no input with `crit`, so `crit` is tested in `verify_tests`
//! instead.

use dpp_crypto::jws::verifier::verify_jws;
use dpp_crypto::jws::{KeyAlgorithm, PublicKey};

use super::super::super::VerifyOptions;
use super::super::super::{JoseError, verify_credential, verify_presentation};
use crate::test_support::NOT_REQUIRED;
use crate::test_support::{public_key, token};

/// The suite's verdict for a verification: success when the function returned.
fn harness<T>(result: &Result<T, JoseError>) -> &'static str {
    if result.is_ok() { "success" } else { "failure" }
}

fn signature_is_valid(file: &str, key: &PublicKey) -> bool {
    verify_jws(&token(file), key).expect("well-formed")
}

#[test]
fn row_6_a_basic_credential_verifies() {
    let result = verify_credential(
        &token("credential-jose-minimal.txt"),
        &public_key("p256"),
        VerifyOptions::new(),
    );

    assert_eq!(harness(&result), "success");
    let credential = result.expect("verified");
    assert_eq!(credential["issuer"], "https://example.issuer/vc-jose-cose");
    assert_eq!(credential["type"][1], "ExampleAlumniCredential");
}

/// Row 7: the presentation's outer token is ES384, and its three envelopes are a
/// `vc+jwt` signed under P-256, a `vc+sd-jwt` and a `vc+cose`. The outer token verifies
/// under the P-384 key. The envelopes are listed and none is verified: verifying a
/// presentation does not verify the credentials in it.
#[test]
fn row_7_the_presentation_verifies_and_lists_its_three_envelopes_unverified() {
    let result = verify_presentation(
        &token("presentation-jose-multiple.txt"),
        &public_key("p384"),
        VerifyOptions::new(),
    );

    assert_eq!(harness(&result), "success");
    let presentation = result.expect("verified");
    let listed: Vec<_> = presentation
        .envelopes()
        .iter()
        .map(|e| (e.index, e.media_type.as_str(), e.kid.as_deref()))
        .collect();
    assert_eq!(
        listed,
        [
            (
                0,
                "application/vc+jwt",
                Some("73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU")
            ),
            (
                1,
                "application/vc+sd-jwt",
                Some("96qBnDpvS6x8YSCH_xmnLlXZMkDhHBr8lj99u5aog7s")
            ),
            (2, "application/vc+cose", None),
        ]
    );
}

/// Row 7's presentation carries an `exp` of 1734397450, 17 December 2024, and the
/// suite names no verification time. The test above passes no time, as this
/// repository's harness for the suite does, so `exp` is not read there. Held to a
/// clock, the presentation verifies until that second and is refused from it on, so
/// under today's clock it is refused.
#[test]
fn row_7_is_refused_from_its_exp_on() {
    const EXP: i64 = 1_734_397_450;
    let presentation = token("presentation-jose-multiple.txt");
    let key = public_key("p384");

    assert!(verify_presentation(&presentation, &key, VerifyOptions::new().at(EXP - 1)).is_ok());
    assert!(matches!(
        verify_presentation(&presentation, &key, VerifyOptions::new().at(EXP)),
        Err(JoseError::Expired)
    ));
}

#[test]
fn row_7_the_vc_jwt_envelope_verifies_under_p256() {
    let presentation = verify_presentation(
        &token("presentation-jose-multiple.txt"),
        &public_key("p384"),
        VerifyOptions::new(),
    )
    .expect("verified");

    let first = &presentation.envelopes()[0];
    assert_eq!(
        first.kid.as_deref(),
        Some("73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU")
    );
    let credential = first
        .verify(&public_key("p256"), VerifyOptions::new(), &NOT_REQUIRED)
        .expect("the vc+jwt verifies under P-256");
    assert_eq!(credential["issuer"], "https://example.issuer/vc-jose-cose");
}

/// The second envelope is a `vc+sd-jwt` signed under P-384, and the third a `vc+cose` signed
/// under P-256, whose base64 has no padding and ends in a line feed. Under P-256 for every
/// envelope the second is refused for its algorithm, and under P-384 for it, all three verify.
#[test]
fn row_7_verify_credentials_verifies_all_three_envelopes_under_their_keys() {
    let presentation = verify_presentation(
        &token("presentation-jose-multiple.txt"),
        &public_key("p384"),
        VerifyOptions::new(),
    )
    .expect("verified");
    let (p256, p384) = (public_key("p256"), public_key("p384"));

    let one_key = presentation
        .verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |_| Some(p256.clone()));
    let result = presentation.verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |envelope| {
        Some(if envelope.index == 1 {
            p384.clone()
        } else {
            p256.clone()
        })
    });

    assert_eq!(
        one_key.expect_err("refused"),
        JoseError::CredentialRefused {
            index: 1,
            source: Box::new(JoseError::AlgorithmMismatch {
                header_alg: Some("ES384".to_owned()),
                key_alg: KeyAlgorithm::EcdsaP256,
            }),
        }
    );
    let credentials = result.expect("all three verify");
    assert_eq!(credentials.len(), 3);
    assert_eq!(
        credentials[2]["issuer"],
        "https://example.issuer/vc-jose-cose"
    );
}

#[test]
fn row_7_with_no_key_for_the_first_envelope_the_credentials_are_refused() {
    let presentation = verify_presentation(
        &token("presentation-jose-multiple.txt"),
        &public_key("p384"),
        VerifyOptions::new(),
    )
    .expect("verified");

    let result = presentation.verify_credentials(VerifyOptions::new(), &NOT_REQUIRED, |_| None);

    assert_eq!(
        result.expect_err("refused"),
        JoseError::CredentialKeyUnknown {
            index: 0,
            kid: Some("73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU".to_owned()),
        }
    );
}

#[test]
fn row_8_a_credential_whose_iss_is_its_issuer_verifies() {
    let result = verify_credential(
        &token("credential-issuer-match-signed.txt"),
        &public_key("ed25519"),
        VerifyOptions::new(),
    );

    assert_eq!(harness(&result), "success");
    let credential = result.expect("verified");
    assert_eq!(credential["iss"], credential["issuer"]);
}

/// Row 9 is named for unknown extensions and is refused for another reason: the
/// token is ES512 and the key is Ed25519. It has no `crit` and no member this
/// verifier would object to, and the same token verifies under the P-521 key.
#[test]
fn row_9_is_refused_because_its_algorithm_is_not_the_keys() {
    let file = "credential-jose-unknown-extensions.txt";
    let result = verify_credential(&token(file), &public_key("ed25519"), VerifyOptions::new());

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::AlgorithmMismatch {
            header_alg: Some("ES512".to_owned()),
            key_alg: KeyAlgorithm::Ed25519,
        }
    );
    assert!(verify_credential(&token(file), &public_key("p521"), VerifyOptions::new()).is_ok());
}

#[test]
fn rows_10_and_11_json_that_is_not_a_token_is_refused() {
    let ed25519 = public_key("ed25519");
    for file in ["credential-minimal.json", "presentation-single.json"] {
        let raw = crate::test_support::file(file);
        let credential = verify_credential(raw, &ed25519, VerifyOptions::new());
        let presentation = verify_presentation(raw, &ed25519, VerifyOptions::new());

        assert_eq!(harness(&credential), "failure", "{file}");
        assert_eq!(harness(&presentation), "failure", "{file}");
        assert_eq!(
            credential.expect_err("refused"),
            JoseError::NotCompactJws,
            "{file}"
        );
        assert_eq!(
            presentation.expect_err("refused"),
            JoseError::NotCompactJws,
            "{file}"
        );
    }
}

#[test]
fn row_12_a_bad_signature_is_refused() {
    let result = verify_credential(
        &token("credential-jose-bad-signature.txt"),
        &public_key("ed25519"),
        VerifyOptions::new(),
    );

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::SignatureDoesNotVerify
    );
}

#[test]
fn row_13_a_credential_with_a_bad_media_type_is_refused_for_its_typ() {
    let key = public_key("ed25519");
    assert!(signature_is_valid(
        "credential-jose-bad-media-type.txt",
        &key
    ));

    let result = verify_credential(
        &token("credential-jose-bad-media-type.txt"),
        &key,
        VerifyOptions::new(),
    );

    assert_eq!(harness(&result), "failure");
    // `cty` is bad too, and `typ` is read first.
    assert_eq!(
        result.expect_err("refused"),
        JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"bad+typ\"".to_owned()),
            expected: "vc+jwt",
        }
    );
}

#[test]
fn row_14_a_presentation_with_a_bad_media_type_is_refused_for_its_typ() {
    let key = public_key("ed25519");
    assert!(signature_is_valid(
        "presentation-jose-bad-media-type.txt",
        &key
    ));

    let result = verify_presentation(
        &token("presentation-jose-bad-media-type.txt"),
        &key,
        VerifyOptions::new(),
    );

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::WrongMediaType {
            member: "typ",
            found: Some("\"bad+typ\"".to_owned()),
            expected: "vp+jwt",
        }
    );
}

#[test]
fn row_15_a_credential_with_vc_and_vp_claims_is_refused_for_them() {
    let key = public_key("ed25519");
    assert!(signature_is_valid("credential-jose-vc-vp-claims.txt", &key));

    let result = verify_credential(
        &token("credential-jose-vc-vp-claims.txt"),
        &key,
        VerifyOptions::new(),
    );

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::ReservedClaim { name: "vc" }
    );
}

/// The outer token is valid, typed `vp+jwt`, and carries one envelope whose `id` is
/// `not-a-credential`. The envelope's shape is refused by `verify_presentation`, so no
/// presentation is returned and no credential is looked at.
#[test]
fn row_16_an_envelope_that_is_not_a_data_url_is_refused() {
    let key = public_key("ed25519");
    assert!(signature_is_valid(
        "presentation-jose-bad-credential.txt",
        &key
    ));

    let result = verify_presentation(
        &token("presentation-jose-bad-credential.txt"),
        &key,
        VerifyOptions::new(),
    );

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::MalformedEnvelope {
            index: 0,
            reason: "id is not a data: URL".to_owned(),
        }
    );
}
