//! The harness run as the suite runs it: arguments in, a verdict file out.

use super::*;

/// A scratch directory holding the input and key files of one run.
struct Run(PathBuf);

impl Run {
    fn new(label: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("vc-jose-cose-suite-{label}-{}", std::process::id()));
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

/// A feature the suite does not have is an error and writes nothing, even when an earlier
/// run left a `success` at the same path.
#[test]
fn an_unknown_feature_is_an_error_and_clears_a_stale_file() {
    let run = Run::new("stale");
    let input = run.write("in.json", fixture!("credential-minimal.json"));
    let key = run.write("key.json", P256);
    std::fs::write(run.output(), r#"{"result":"success"}"#).expect("a stale file");

    let refusal = crate::answer(run.args("issue", "credential_jwt", &input, &key))
        .expect_err("no verdict for a feature that is not the suite's");

    assert!(refusal.contains("credential_jwt"), "{refusal}");
    assert!(!run.output().exists(), "a file was left");
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
