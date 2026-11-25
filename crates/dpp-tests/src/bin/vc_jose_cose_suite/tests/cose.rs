//! Rows 27 to 35: the COSE features.

use super::*;

fn verdict(feature: Feature, token: &str, key: &str) -> Verdict {
    verify(feature, token, key).expect("a verdict")
}

/// Rows 27 and 28: issued as base64 text in the form the suite's inputs have, and read
/// back by the harness's own verification.
#[test]
fn rows_27_and_28_issue_base64_text_that_verifies_back() {
    let rows = [
        (
            Feature::CredentialCose,
            fixture!("credential-minimal.json"),
            DID_ED25519,
        ),
        (
            Feature::PresentationCose,
            fixture!("presentation-single.json"),
            P384,
        ),
    ];
    for (feature, document, key) in rows {
        let issued = issue(feature, document, key, &[]).expect("a verdict");
        let Verdict::Success(Some(text)) = issued else {
            panic!("not issued: {issued:?}")
        };

        // Standard alphabet, no padding, no line break: the form of the suite's own files.
        assert!(
            text.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'+' || b == b'/'),
            "{text}"
        );
        // A tagged COSE_Sign1: tag 18, then an array of four.
        assert!(text.starts_with("0oR"), "{text}");
        assert_eq!(verdict(feature, &text, key), Verdict::Success(None));
        assert_eq!(
            verdict(feature, &format!("{text}\n"), key),
            Verdict::Success(None),
            "a final newline is no part of the text"
        );
    }
}

/// Rows 29 to 35 with the keys the suite maps them to, each with the verdict it expects.
#[test]
fn the_verification_rows_get_the_verdict_the_suite_expects() {
    let rows = [
        (
            "29",
            Feature::CredentialCose,
            fixture!("credential-cose-minimal.txt"),
            P256,
            true,
        ),
        (
            "30",
            Feature::CredentialCose,
            fixture!("credential-cose-minimal-not-base64.txt"),
            P256,
            false,
        ),
        (
            "31",
            Feature::PresentationCose,
            fixture!("presentation-cose-single.txt"),
            P384,
            true,
        ),
        (
            "32",
            Feature::CredentialCose,
            fixture!("credential-cose-bad-signature.txt"),
            ED25519,
            false,
        ),
        (
            "33",
            Feature::CredentialCose,
            fixture!("credential-cose-bad-media-type.txt"),
            P384,
            false,
        ),
        (
            "34",
            Feature::PresentationCose,
            fixture!("presentation-cose-bad-media-type.txt"),
            P384,
            false,
        ),
        (
            "35",
            Feature::PresentationCose,
            fixture!("presentation-cose-bad-credential.txt"),
            P256,
            false,
        ),
    ];
    for (row, feature, token, key, expected) in rows {
        let verdict = verdict(feature, token, key);

        assert_eq!(is_success(&verdict), expected, "row {row}: {verdict:?}");
    }
}

/// Row 30's hex is also base64, so it is the library that refuses it, for not being a
/// `COSE_Sign1`. No test for hex stands between the file and the library.
#[test]
fn row_30_is_refused_by_the_library_and_not_for_being_hex() {
    let Verdict::Failure(reason) = verdict(
        Feature::CredentialCose,
        fixture!("credential-cose-minimal-not-base64.txt"),
        P256,
    ) else {
        panic!("verified")
    };

    assert!(reason.contains("COSE_Sign1"), "{reason}");
}

/// Text that is not base64 at all is the input's fault, so a refusal and not a missing
/// verdict, as a JSON document in place of a token is.
#[test]
fn text_that_is_not_base64_is_a_refusal() {
    for text in ["not base64!", "a-b_c", "A", ""] {
        assert!(
            matches!(
                verdict(Feature::CredentialCose, text, P256),
                Verdict::Failure(_)
            ),
            "{text:?}"
        );
    }
    let Verdict::Failure(reason) = verdict(Feature::PresentationCose, "a-b_c", P256) else {
        panic!("verified")
    };
    assert!(reason.contains("not base64"), "{reason}");
}

/// A credential and a presentation are not taken for each other.
#[test]
fn a_credential_is_not_a_presentation() {
    assert!(matches!(
        verdict(
            Feature::PresentationCose,
            fixture!("credential-cose-minimal.txt"),
            P256
        ),
        Verdict::Failure(_)
    ));
}

/// `--sd` stays refused for the COSE features, as for the JOSE ones, and leaves no file.
#[test]
fn sd_given_to_a_cose_issuance_is_an_error_with_no_file() {
    for feature in ["credential_cose", "presentation_cose"] {
        let dir = std::env::temp_dir().join(format!(
            "vc-jose-cose-suite-sd-cose-{feature}-{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let write = |name: &str, text: &str| {
            let path = dir.join(name);
            std::fs::write(&path, text).expect("written");
            path.display().to_string()
        };
        let input = write("in.json", fixture!("credential-minimal.json"));
        let key = write("key.json", P256);
        let output = dir.join("out.json");
        std::fs::write(&output, r#"{"result":"success"}"#).expect("a stale file");
        let args: Vec<String> = [
            "issue",
            "--input",
            &input,
            "--key",
            &key,
            "--feature",
            feature,
            "--sd",
            "[]",
            "--output",
            &output.display().to_string(),
        ]
        .map(str::to_owned)
        .to_vec();

        let refusal = crate::answer(args).expect_err("no verdict");

        assert!(refusal.contains("--sd"), "{refusal}");
        assert!(!output.exists(), "{feature}: a file was left");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The command line of row 27 writes the text the suite's checker reads.
#[test]
fn the_command_line_of_row_27_writes_the_text() {
    let dir = std::env::temp_dir().join(format!("vc-jose-cose-suite-cose-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let write = |name: &str, text: &str| {
        let path = dir.join(name);
        std::fs::write(&path, text).expect("written");
        path.display().to_string()
    };
    let input = write("in.json", fixture!("credential-minimal.json"));
    let key = write("key.json", DID_ED25519);
    let output = dir.join("out.json").display().to_string();
    let args: Vec<String> = [
        "issue",
        "--input",
        &input,
        "--key",
        &key,
        "--feature",
        "credential_cose",
        "--output",
        &output,
    ]
    .map(str::to_owned)
    .to_vec();

    crate::answer(args).expect("answered");

    let written: Value =
        serde_json::from_str(&std::fs::read_to_string(&output).expect("a file")).expect("JSON");
    assert_eq!(written["result"], "success");
    assert!(
        written["data"]
            .as_str()
            .is_some_and(|t| t.starts_with("0oR"))
    );
    let _ = std::fs::remove_dir_all(&dir);
}
