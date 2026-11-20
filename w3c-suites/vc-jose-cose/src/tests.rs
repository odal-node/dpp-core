//! The harness against the suite's own inputs, vendored once in `dpp-vc`.

use std::path::PathBuf;

use serde_json::Value;

use crate::feature::Feature;
use crate::run::{Verdict, issue, verify};

macro_rules! fixture {
    ($name:literal) => {
        include_str!(concat!(
            "../../../crates/dpp-vc/src/jose/fixtures/w3c-vc-jose-cose/",
            $name
        ))
    };
}

const P256: &str = fixture!("vm-p256.json");
const P384: &str = fixture!("vm-p384.json");
const ED25519: &str = fixture!("vm-ed25519.json");

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

mod command_line {
    use super::*;

    /// A scratch directory holding the input and key files of one run.
    struct Run(PathBuf);

    impl Run {
        fn new(label: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("vc-jose-cose-suite-{label}-{}", std::process::id()));
            std::fs::create_dir_all(&dir).expect("scratch dir");
            Self(dir)
        }

        fn write(&self, name: &str, text: &str) -> String {
            let path = self.0.join(name);
            std::fs::write(&path, text).expect("written");
            path.display().to_string()
        }

        fn args(&self, command: &str, feature: &str, input: &str, key: &str) -> Vec<String> {
            let output = self.0.join("out.json").display().to_string();
            [
                command,
                "--input",
                input,
                "--key",
                key,
                "--feature",
                feature,
                "--output",
                &output,
            ]
            .map(str::to_owned)
            .to_vec()
        }

        fn output(&self) -> PathBuf {
            self.0.join("out.json")
        }
    }

    impl Drop for Run {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_verdict_is_written_for_a_supported_feature() {
        let run = Run::new("written");
        let input = run.write("in.json", fixture!("credential-minimal.json"));
        let key = run.write("key.json", P256);

        crate::answer(run.args("issue", "credential_jose", &input, &key)).expect("answered");

        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(run.output()).expect("a file"))
                .expect("JSON");
        assert_eq!(written["result"], "success");
        assert!(
            written["data"]
                .as_str()
                .is_some_and(|t| t.split('.').count() == 3)
        );
    }

    /// The two features the harness does not implement write nothing, even when an
    /// earlier run left a `success` at the same path.
    #[test]
    fn an_unimplemented_feature_is_an_error_and_clears_a_stale_file() {
        let run = Run::new("stale");
        let input = run.write("in.json", fixture!("credential-minimal.json"));
        let key = run.write("key.json", P256);
        std::fs::write(run.output(), r#"{"result":"success"}"#).expect("a stale file");

        for feature in ["credential_cose", "presentation_cose"] {
            let refusal = crate::answer(run.args("issue", feature, &input, &key))
                .expect_err("no verdict for a feature that is not implemented");
            assert!(refusal.contains(feature), "{refusal}");
            assert!(!run.output().exists(), "{feature} left a file");
        }
    }

    #[test]
    fn a_missing_input_file_is_an_error_with_no_file() {
        let run = Run::new("missing");
        let key = run.write("key.json", P256);
        let missing = run.0.join("absent.txt").display().to_string();

        let refusal = crate::answer(run.args("verify", "credential_jose", &missing, &key))
            .expect_err("no verdict");

        assert!(refusal.contains("cannot read"), "{refusal}");
        assert!(!run.output().exists());
    }

    /// `--sd` given to a JOSE issuance is an unusable input, even as an empty list. It is
    /// refused before any file is read or written, so a file left by an earlier run is
    /// cleared and nothing replaces it.
    #[test]
    fn sd_given_to_a_jose_issuance_is_an_error_with_no_file() {
        let run = Run::new("sd-jose");
        let input = run.write("in.json", fixture!("credential-minimal.json"));
        let key = run.write("key.json", P256);
        std::fs::write(run.output(), r#"{"result":"success"}"#).expect("a stale file");
        let mut args = run.args("issue", "credential_jose", &input, &key);
        args.extend(["--sd".to_owned(), "[]".to_owned()]);

        let refusal = crate::answer(args).expect_err("no verdict");

        assert!(refusal.contains("--sd"), "{refusal}");
        assert!(!run.output().exists(), "a file was left");
    }

    /// `--sd` given to a verification is an unusable input, even as an empty list, for the
    /// same reason as an issuance under a JOSE feature.
    #[test]
    fn sd_given_to_a_verification_is_an_error_with_no_file() {
        let run = Run::new("sd-verify");
        let token = run.write("token.txt", fixture!("credential-sdjwt-selective.txt"));
        let key = run.write("key.json", P384);
        std::fs::write(run.output(), r#"{"result":"success"}"#).expect("a stale file");
        let mut args = run.args("verify", "credential_sdjwt", &token, &key);
        args.extend(["--sd".to_owned(), "[]".to_owned()]);

        let refusal = crate::answer(args).expect_err("no verdict");

        assert!(refusal.contains("--sd"), "{refusal}");
        assert!(!run.output().exists(), "a file was left");
    }
}

mod sd_jwt {
    use super::*;
    use crate::paths::parse_sd;
    use dpp_crypto::sd_jwt::{Disclosure, SdJwt};

    const P521: &str = fixture!("vm-p521.json");

    fn disclose(paths: &str) -> Vec<dpp_crypto::sd_jwt::ClaimPath> {
        parse_sd(paths).expect("paths")
    }

    /// Rows 17 to 19 with the `--sd` the suite sends, issued and verified back by the
    /// harness's own verification under the same key.
    #[test]
    fn rows_17_to_19_issue_a_token_that_verifies_back() {
        let rows = [
            (
                Feature::CredentialSdJwt,
                fixture!("credential-selective.json"),
                P384,
                r#"["credentialSubject.firstName","credentialSubject.lastName"]"#,
            ),
            (
                Feature::CredentialSdJwt,
                fixture!("credential-nested-selective.json"),
                P521,
                r#"["credentialSubject.address.street","credentialSubject.address.city","credentialSubject.phoneNumbers[0]"]"#,
            ),
            (
                Feature::PresentationSdJwt,
                fixture!("presentation-selective.json"),
                P384,
                r#"["holder","verifiableCredential[0]"]"#,
            ),
        ];
        for (feature, document, key, sd) in rows {
            let verdict = issue(feature, document, key, &disclose(sd)).expect("a verdict");
            let Verdict::Success(Some(token)) = verdict else {
                panic!("not issued: {verdict:?}")
            };

            assert!(token.ends_with('~'), "{token}");
            assert_eq!(
                token.split('~').next().map(|t| t.split('.').count()),
                Some(3)
            );
            assert_eq!(
                verify(feature, &token, key).expect("a verdict"),
                Verdict::Success(None),
                "{sd}"
            );
        }
    }

    #[test]
    fn the_paths_of_row_17_conceal_exactly_those_claims() {
        let verdict = issue(
            Feature::CredentialSdJwt,
            fixture!("credential-selective.json"),
            P384,
            &disclose(r#"["credentialSubject.firstName","credentialSubject.lastName"]"#),
        )
        .expect("a verdict");
        let Verdict::Success(Some(token)) = verdict else {
            panic!("not issued")
        };

        assert_eq!(
            token.split('~').count(),
            4,
            "two Disclosures and the closing ~"
        );
        // What a verifier reads when the holder discloses nothing: the Issuer-signed
        // payload, with every concealed claim left out.
        let payload = SdJwt::parse(&token)
            .and_then(|sd_jwt| sd_jwt.present(&[]))
            .and_then(|sd_jwt| sd_jwt.disclosed_payload())
            .map(Value::Object)
            .expect("a payload");
        assert!(
            payload["credentialSubject"].get("firstName").is_none(),
            "{payload}"
        );
        assert!(
            payload["credentialSubject"].get("lastName").is_none(),
            "{payload}"
        );

        let fixture: Value =
            serde_json::from_str(fixture!("credential-selective.json")).expect("JSON");
        let disclosed: Vec<Disclosure> = token
            .split('~')
            .skip(1)
            .take(2)
            .map(|encoded| Disclosure::parse(encoded).expect("a Disclosure"))
            .collect();
        let mut names: Vec<&str> = disclosed.iter().filter_map(|d| d.claim_name()).collect();
        names.sort_unstable();
        assert_eq!(names, ["firstName", "lastName"]);
        for disclosure in &disclosed {
            let name = disclosure.claim_name().expect("a claim name");
            assert_eq!(
                disclosure.claim_value(),
                &fixture["credentialSubject"][name]
            );
        }
    }

    #[test]
    fn the_verification_rows_get_the_verdict_the_suite_expects() {
        let rows = [
            (
                Feature::CredentialSdJwt,
                fixture!("credential-sdjwt-selective.txt"),
                P384,
                true,
            ),
            (
                Feature::CredentialSdJwt,
                fixture!("credential-sdjwt-nested.txt"),
                P521,
                true,
            ),
            (
                Feature::PresentationSdJwt,
                fixture!("presentation-sdjwt-selective.txt"),
                P384,
                true,
            ),
            (
                Feature::CredentialSdJwt,
                fixture!("credential-sdjwt-bad-signature.txt"),
                P384,
                false,
            ),
            (
                Feature::CredentialSdJwt,
                fixture!("credential-sdjwt-bad-media-type.txt"),
                ED25519,
                false,
            ),
            (
                Feature::PresentationSdJwt,
                fixture!("presentation-sdjwt-bad-media-type.txt"),
                ED25519,
                false,
            ),
            (
                Feature::PresentationSdJwt,
                fixture!("presentation-sdjwt-bad-credential.txt"),
                P521,
                false,
            ),
        ];
        for (feature, token, key, expected) in rows {
            let verdict = verify(feature, token, key).expect("a verdict");

            assert_eq!(is_success(&verdict), expected, "{feature:?} {verdict:?}");
        }
    }

    #[test]
    fn a_refusal_of_an_issuance_is_a_failure_with_its_reason() {
        let verdict = issue(
            Feature::CredentialSdJwt,
            fixture!("credential-selective.json"),
            P384,
            &disclose(r#"["type"]"#),
        )
        .expect("a verdict");

        let Verdict::Failure(reason) = verdict else {
            panic!("issued")
        };
        assert!(reason.contains("type"), "{reason}");
    }

    /// `--sd` with a JOSE feature is not a verdict: signing without concealing what was
    /// asked to be concealed would answer another question.
    #[test]
    fn paths_given_to_a_jose_feature_are_an_error() {
        let result = issue(
            Feature::CredentialJose,
            fixture!("credential-minimal.json"),
            P256,
            &disclose(r#"["credentialSubject.name"]"#),
        );

        assert!(result.is_err());
    }

    #[test]
    fn the_command_line_of_row_17_writes_the_token() {
        let dir =
            std::env::temp_dir().join(format!("vc-jose-cose-suite-sd-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("scratch dir");
        let write = |name: &str, text: &str| {
            let path = dir.join(name);
            std::fs::write(&path, text).expect("written");
            path.display().to_string()
        };
        let input = write("in.json", fixture!("credential-selective.json"));
        let key = write("key.json", P384);
        let output = dir.join("out.json").display().to_string();
        let sd = r#"["credentialSubject.firstName","credentialSubject.lastName"]"#;
        let command = |sd: Option<&str>| {
            let mut args: Vec<String> = [
                "issue",
                "--input",
                &input,
                "--key",
                &key,
                "--feature",
                "credential_sdjwt",
                "--output",
                &output,
            ]
            .map(str::to_owned)
            .to_vec();
            if let Some(sd) = sd {
                args.extend(["--sd".to_owned(), sd.to_owned()]);
            }
            args
        };

        crate::answer(command(Some(sd))).expect("answered");
        let written: Value =
            serde_json::from_str(&std::fs::read_to_string(&output).expect("a file")).expect("JSON");
        let bad_sd = crate::answer(command(Some("not json")));
        let still_there = std::path::Path::new(&output).exists();

        assert_eq!(written["result"], "success");
        assert!(written["data"].as_str().is_some_and(|t| t.ends_with('~')));
        assert!(bad_sd.is_err());
        assert!(
            !still_there,
            "an unusable --sd left the earlier file in place"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
