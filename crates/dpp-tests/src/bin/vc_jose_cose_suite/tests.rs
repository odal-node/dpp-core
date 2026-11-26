//! The harness against the suite's own inputs, vendored once in `dpp-vc`.

use std::path::PathBuf;

use serde_json::Value;

use crate::feature::Feature;
use crate::run::{Verdict, issue, verify};

// Named from the crate root, since the modules under `tests/` invoke it from their own
// directory.
macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../dpp-vc/src/jose/fixtures/w3c-vc-jose-cose/",
            $name
        ))
    };
}

const P256: &str = fixture!("vm-p256.json");
const P384: &str = fixture!("vm-p384.json");
const ED25519: &str = fixture!("vm-ed25519.json");
const DID_ED25519: &str = fixture!("vm-did-ed25519.json");

fn is_success(verdict: &Verdict) -> bool {
    matches!(verdict, Verdict::Success(_))
}

#[test]
fn row_1_issues_a_token_that_verifies_under_the_public_key() {
    let verdict = issue(
        Feature::CredentialJose,
        fixture!("credential-minimal.json"),
        P256,
        &[],
    )
    .expect("a verdict");
    let Verdict::Success(Some(token)) = verdict else {
        panic!("not issued: {verdict:?}")
    };

    let back = verify(Feature::CredentialJose, &token, P256).expect("a verdict");
    assert_eq!(back, Verdict::Success(None));
}

#[test]
fn row_5_is_refused_at_issuance_with_the_undeclared_members_named() {
    let verdict = issue(
        Feature::CredentialJose,
        fixture!("credential-unknown-extensions.json"),
        fixture!("vm-p521.json"),
        &[],
    )
    .expect("a verdict");

    let Verdict::Failure(reason) = verdict else {
        panic!("issued")
    };
    assert!(reason.contains("badExtension"), "{reason}");
}

#[test]
fn a_verification_row_that_expects_success_gets_it_with_no_clock() {
    let credential = verify(
        Feature::CredentialJose,
        fixture!("credential-jose-minimal.txt"),
        P256,
    )
    .expect("a verdict");
    // Row 7's token expired in December 2024 and is still to verify.
    let presentation = verify(
        Feature::PresentationJose,
        fixture!("presentation-jose-multiple.txt"),
        P384,
    )
    .expect("a verdict");

    assert!(is_success(&credential));
    assert!(is_success(&presentation));
}

#[test]
fn a_verification_row_that_expects_a_refusal_gets_one() {
    // Row 10 hands the verifier an unsecured credential in place of a token.
    let unsecured = verify(
        Feature::CredentialJose,
        fixture!("credential-minimal.json"),
        ED25519,
    )
    .expect("a verdict");
    let bad_signature = verify(
        Feature::CredentialJose,
        fixture!("credential-jose-bad-signature.txt"),
        ED25519,
    )
    .expect("a verdict");

    assert!(matches!(unsecured, Verdict::Failure(_)));
    assert!(matches!(bad_signature, Verdict::Failure(_)));
}

#[test]
fn a_token_file_that_ends_in_a_newline_is_read_as_the_token() {
    let with_newline = format!("{}\n", fixture!("credential-jose-minimal.txt").trim_end());

    let verdict = verify(Feature::CredentialJose, &with_newline, P256).expect("a verdict");

    assert!(is_success(&verdict));
}

/// A key file that is not a key is the harness unable to ask, not the library
/// refusing: no verdict, so no file for the suite to score.
#[test]
fn an_unusable_key_file_is_an_error_and_not_a_verdict() {
    let no_secret = r#"{"id": "https://example.issuer/k", "publicKeyJwk": {}}"#;
    let no_public = r#"{"id": "https://example.issuer/k", "secretKeyJwk": {}}"#;

    assert!(
        issue(
            Feature::CredentialJose,
            fixture!("credential-minimal.json"),
            no_secret,
            &[]
        )
        .is_err()
    );
    assert!(issue(Feature::CredentialJose, "not json", P256, &[]).is_err());
    assert!(verify(Feature::CredentialJose, "a.b.c", no_public).is_err());
    assert!(verify(Feature::CredentialJose, "a.b.c", "not json").is_err());
}

#[test]
fn a_verdict_is_the_json_the_suite_reads() {
    let issued: Value = Verdict::Success(Some("a.b.c".into())).to_json();
    let verified: Value = Verdict::Success(None).to_json();
    let refused: Value = Verdict::Failure("why".into()).to_json();

    assert_eq!(
        issued,
        serde_json::json!({"result": "success", "data": "a.b.c"})
    );
    assert_eq!(verified, serde_json::json!({"result": "success"}));
    assert_eq!(
        refused,
        serde_json::json!({"result": "failure", "reason": "why"})
    );
}

mod command_line;
mod cose;
mod sd_jwt;
