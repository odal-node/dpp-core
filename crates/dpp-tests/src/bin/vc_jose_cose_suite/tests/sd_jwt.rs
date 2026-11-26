//! Rows 17 to 26: the SD-JWT features.

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

    let fixture: Value = serde_json::from_str(fixture!("credential-selective.json")).expect("JSON");
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
    let dir = std::env::temp_dir().join(format!("vc-jose-cose-suite-sd-{}", std::process::id()));
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
