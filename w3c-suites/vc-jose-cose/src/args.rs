//! The command line the suite builds, in `tests/test-util.js`:
//!
//! ```text
//! <command> --input <file> --key <file> --feature <name> [--sd <json>] --output <file>
//! ```

use std::path::PathBuf;

/// What the suite asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Secure the document in `--input` and write the token.
    Issue,
    /// Verify the token in `--input`.
    Verify,
}

/// A parsed command line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    pub command: Command,
    /// The unsecured document to issue, or the token to verify.
    pub input: PathBuf,
    /// A verification method: a `JsonWebKey` with `publicKeyJwk` and `secretKeyJwk`.
    pub key: PathBuf,
    /// One of the suite's six feature names, as given.
    pub feature: String,
    /// Where the verdict is written.
    pub output: PathBuf,
}

/// Parse the arguments after the program name. The error is a usage message.
///
/// `--sd` is accepted and ignored: the suite passes it only to SD-JWT rows, and a
/// feature this harness does not implement is refused later, by name.
pub fn parse<I: IntoIterator<Item = String>>(args: I) -> Result<Args, String> {
    let mut args = args.into_iter();
    let command = match args.next().as_deref() {
        Some("issue") => Command::Issue,
        Some("verify") => Command::Verify,
        Some(other) => {
            return Err(format!(
                "unknown command `{other}`: expected issue or verify"
            ));
        }
        None => return Err("no command: expected issue or verify".to_owned()),
    };

    let (mut input, mut key, mut feature, mut output) = (None, None, None, None);
    while let Some(flag) = args.next() {
        let slot = match flag.as_str() {
            "--input" => &mut input,
            "--key" => &mut key,
            "--feature" => &mut feature,
            "--output" => &mut output,
            "--sd" => {
                args.next().ok_or_else(|| "--sd needs a value".to_owned())?;
                continue;
            }
            other => return Err(format!("unknown argument `{other}`")),
        };
        let value = args.next().ok_or_else(|| format!("{flag} needs a value"))?;
        if slot.replace(value).is_some() {
            return Err(format!("{flag} given twice"));
        }
    }

    let required =
        |value: Option<String>, flag: &str| value.ok_or_else(|| format!("{flag} is required"));
    Ok(Args {
        command,
        input: PathBuf::from(required(input, "--input")?),
        key: PathBuf::from(required(key, "--key")?),
        feature: required(feature, "--feature")?,
        output: PathBuf::from(required(output, "--output")?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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

    /// An SD-JWT issuance row carries `--sd` with a JSON array; it is read past.
    #[test]
    fn the_sd_flag_is_accepted_and_ignored() {
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
}
