//! The command line the W3C VC-JOSE-COSE test suite runs against `dpp-vc`.
//!
//! The suite starts this program once per row, in a container, with the arguments of
//! `tests/test-util.js`, and reads the JSON file named by `--output`. See `README.md`.
//!
//! Exit status: 0 when a verdict was written, whichever it is (a refusal is a correct
//! answer to a row that expects one), and 2 when none could be, with the reason on
//! stderr and no output file.

use std::process::ExitCode;
use std::{fs, io};

mod args;
mod feature;
mod run;

use args::{Args, Command};
use feature::Feature;

fn main() -> ExitCode {
    match answer(std::env::args().skip(1)) {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("error: {message}");
            ExitCode::from(2)
        }
    }
}

fn answer<I: IntoIterator<Item = String>>(args: I) -> Result<(), String> {
    let args = args::parse(args)?;

    // The suite reads whatever is at `--output`, so a file left by an earlier run
    // would score this one. Nothing is left: either a verdict is written below, or
    // the run is an error with no file.
    match fs::remove_file(&args.output) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(format!("cannot clear {}: {e}", args.output.display())),
    }

    let feature = Feature::parse(&args.feature)?;
    let verdict = judge(&args, feature)?;
    if let run::Verdict::Failure(reason) = &verdict {
        eprintln!("refused: {reason}");
    }
    let text = serde_json::to_string(&verdict.to_json()).map_err(|e| e.to_string())?;
    fs::write(&args.output, text)
        .map_err(|e| format!("cannot write {}: {e}", args.output.display()))
}

fn judge(args: &Args, feature: Feature) -> Result<run::Verdict, String> {
    let read = |path: &std::path::Path| {
        fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
    };
    let (input, key) = (read(&args.input)?, read(&args.key)?);
    match args.command {
        Command::Issue => run::issue(feature, &input, &key),
        Command::Verify => run::verify(feature, &input, &key),
    }
}

#[cfg(test)]
mod tests;
