//! Tests of `args`.

use super::super::*;

fn parse_all(args: &[&str]) -> Result<Args, String> {
    parse(args.iter().map(|a| (*a).to_owned()))
}

/// The exact shape `tests/test-util.js` builds for a verification row.
#[test]
fn the_suites_verify_command_line_parses() {
    let args = parse_all(&[
        "verify",
        "--input",
        "/tests/input/credential-jose-minimal.txt",
        "--key",
        "/tests/input/vm-p256.json",
        "--feature",
        "credential_jose",
        "--output",
        "/tests/output/6-dpp-vc.json",
    ])
    .expect("parses");

    assert_eq!(args.command, Command::Verify);
    assert_eq!(args.feature, "credential_jose");
    assert_eq!(args.key, PathBuf::from("/tests/input/vm-p256.json"));
    assert_eq!(args.output, PathBuf::from("/tests/output/6-dpp-vc.json"));
}

/// An SD-JWT issuance row carries `--sd` with a JSON array, which is kept as given.
#[test]
fn the_sd_flag_is_kept_as_given() {
    let args = parse_all(&[
        "issue",
        "--input",
        "i",
        "--key",
        "k",
        "--feature",
        "credential_sdjwt",
        "--sd",
        r#"["credentialSubject.firstName"]"#,
        "--output",
        "o",
    ])
    .expect("parses");

    assert_eq!(args.command, Command::Issue);
    assert_eq!(args.feature, "credential_sdjwt");
    assert_eq!(
        args.sd.as_deref(),
        Some(r#"["credentialSubject.firstName"]"#)
    );
}

#[test]
fn the_sd_flag_is_absent_when_the_suite_gives_none() {
    let args = parse_all(&[
        "verify",
        "--input",
        "i",
        "--key",
        "k",
        "--feature",
        "credential_jose",
        "--output",
        "o",
    ])
    .expect("parses");

    assert_eq!(args.sd, None);
    assert!(parse_all(&["issue", "--sd"]).is_err());
}

#[test]
fn a_missing_or_unknown_part_is_a_usage_error() {
    assert!(parse_all(&[]).is_err());
    assert!(parse_all(&["validate"]).is_err());
    assert!(parse_all(&["issue", "--input", "i", "--key", "k", "--feature", "f"]).is_err());
    assert!(parse_all(&["issue", "--input"]).is_err());
    assert!(parse_all(&["issue", "--bogus", "x"]).is_err());
    assert!(
        parse_all(&[
            "issue",
            "--input",
            "a",
            "--input",
            "b",
            "--key",
            "k",
            "--feature",
            "f",
            "--output",
            "o"
        ])
        .is_err()
    );
}
