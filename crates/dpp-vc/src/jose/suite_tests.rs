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

use super::test_fixtures::{public_key, token};
use super::{EnvelopeDecision, JoseError, verify_credential, verify_presentation};

/// The suite's verdict for a verification: success when the function returned.
fn harness<T>(result: &Result<T, JoseError>) -> &'static str {
    if result.is_ok() { "success" } else { "failure" }
}

fn signature_is_valid(file: &str, key: &PublicKey) -> bool {
    verify_jws(&token(file), key).expect("well-formed")
}

/// The decision of a caller that verifies nothing and says so.
fn skip_all(_: usize, _: &str, _: Option<&str>) -> EnvelopeDecision {
    EnvelopeDecision::Skip
}

#[test]
fn row_6_a_basic_credential_verifies() {
    let result = verify_credential(
        &token("credential-jose-minimal.txt"),
        &public_key("p256"),
        None,
    );

    assert_eq!(harness(&result), "success");
    let credential = result.expect("verified");
    assert_eq!(credential["issuer"], "https://example.issuer/vc-jose-cose");
    assert_eq!(credential["type"][1], "ExampleAlumniCredential");
}

/// Row 7: the presentation's outer token is ES384, and its three envelopes are a
/// `vc+jwt` signed under P-256, a `vc+sd-jwt` and a `vc+cose`. The suite's key is
/// P-384, so it cannot verify the inner `vc+jwt`, and this function verifies only
/// `vc+jwt`. The caller says what to do with each envelope.
#[test]
fn row_7_skipping_every_envelope_passes_and_reports_all_three() {
    let result = verify_presentation(
        &token("presentation-jose-multiple.txt"),
        &public_key("p384"),
        None,
        skip_all,
    );

    assert_eq!(harness(&result), "success");
    let presentation = result.expect("verified");
    assert!(presentation.verified.is_empty());
    let skipped: Vec<_> = presentation
        .skipped
        .iter()
        .map(|s| (s.index, s.media_type.as_str(), s.kid.as_deref()))
        .collect();
    assert_eq!(
        skipped,
        [
            (
                0,
                "application/vc+jwt",
                Some("73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU")
            ),
            (1, "application/vc+sd-jwt", None),
            (2, "application/vc+cose", None),
        ]
    );
}

#[test]
fn row_7_verifying_the_vc_jwt_under_p256_and_skipping_the_rest() {
    let p256 = public_key("p256");
    let result = verify_presentation(
        &token("presentation-jose-multiple.txt"),
        &public_key("p384"),
        None,
        |index, media_type, kid| match (index, media_type) {
            (0, "application/vc+jwt") => {
                assert_eq!(kid, Some("73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU"));
                EnvelopeDecision::Verify(p256.clone())
            }
            _ => EnvelopeDecision::Skip,
        },
    );

    let presentation = result.expect("verified");
    assert_eq!(presentation.verified.len(), 1);
    assert_eq!(presentation.verified[0].index, 0);
    assert_eq!(
        presentation.verified[0].credential["issuer"],
        "https://example.issuer/vc-jose-cose"
    );
    let skipped: Vec<usize> = presentation.skipped.iter().map(|s| s.index).collect();
    assert_eq!(skipped, [1, 2]);
}

#[test]
fn row_7_with_no_decision_the_presentation_is_refused() {
    let result = verify_presentation(
        &token("presentation-jose-multiple.txt"),
        &public_key("p384"),
        None,
        |_, _, _| EnvelopeDecision::Unknown,
    );

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::EnvelopeNotDecided {
            index: 0,
            media_type: "application/vc+jwt".to_owned(),
            kid: Some("73voMXFNmNlOEpuYCSJlh8eN0dscykoO6gBukgRS1uU".to_owned()),
        }
    );
}

#[test]
fn row_8_a_credential_whose_iss_is_its_issuer_verifies() {
    let result = verify_credential(
        &token("credential-issuer-match-signed.txt"),
        &public_key("ed25519"),
        None,
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
    let result = verify_credential(&token(file), &public_key("ed25519"), None);

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::AlgorithmMismatch {
            header_alg: Some("ES512".to_owned()),
            key_alg: KeyAlgorithm::Ed25519,
        }
    );
    assert!(verify_credential(&token(file), &public_key("p521"), None).is_ok());
}

#[test]
fn rows_10_and_11_json_that_is_not_a_token_is_refused() {
    let ed25519 = public_key("ed25519");
    for file in ["credential-minimal.json", "presentation-single.json"] {
        let raw = super::test_fixtures::file(file);
        let credential = verify_credential(raw, &ed25519, None);
        let presentation = verify_presentation(raw, &ed25519, None, skip_all);

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
        None,
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

    let result = verify_credential(&token("credential-jose-bad-media-type.txt"), &key, None);

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
        None,
        skip_all,
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

    let result = verify_credential(&token("credential-jose-vc-vp-claims.txt"), &key, None);

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::ReservedClaim { name: "vc" }
    );
}

/// The outer token is valid, typed `vp+jwt`, and carries one envelope whose `id` is
/// `not-a-credential`. The caller is never asked about it: shape errors are refused
/// whatever the closure would say.
#[test]
fn row_16_an_envelope_that_is_not_a_data_url_is_refused() {
    let key = public_key("ed25519");
    assert!(signature_is_valid(
        "presentation-jose-bad-credential.txt",
        &key
    ));

    let mut asked = false;
    let result = verify_presentation(
        &token("presentation-jose-bad-credential.txt"),
        &key,
        None,
        |_, _, _| {
            asked = true;
            EnvelopeDecision::Skip
        },
    );

    assert_eq!(harness(&result), "failure");
    assert_eq!(
        result.expect_err("refused"),
        JoseError::MalformedEnvelope {
            index: 0,
            reason: "id is not a data: URL".to_owned(),
        }
    );
    assert!(!asked);
}
