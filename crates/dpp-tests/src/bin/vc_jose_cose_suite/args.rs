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
    /// The paths to conceal: the JSON array `--sd` carried, as given. Only the SD-JWT
    /// rows that issue have one.
    pub sd: Option<String>,
    /// Where the verdict is written.
    pub output: PathBuf,
}

/// Parse the arguments after the program name. The error is a usage message.
///
/// `--sd` is kept as given. `judge` refuses it unless the command issues an SD-JWT feature,
/// which is the only kind of row the suite passes it to.
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

    let (mut input, mut key, mut feature, mut output, mut sd) = (None, None, None, None, None);
    while let Some(flag) = args.next() {
        let slot = match flag.as_str() {
            "--input" => &mut input,
            "--key" => &mut key,
            "--feature" => &mut feature,
            "--output" => &mut output,
            "--sd" => &mut sd,
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
        sd,
        output: PathBuf::from(required(output, "--output")?),
    })
}

#[cfg(test)]
mod tests;
