//! The repository paths a vocabulary record cites exist.
//!
//! A record's `finding` says what was compared with what: a standard's attribute
//! list against `BatteryData`, a definition against `Gtin`, a write path against
//! `Passport`. It names the file, so a reader can check the claim. Four of those
//! files moved when the sector and domain modules were renamed, and six records
//! pointed at nothing for weeks without any check noticing. A claim that cannot be
//! found cannot be re-read, and re-reading is the whole discipline of that crate.
//!
//! This holds only that the path exists. It cannot say the claim is still true of
//! what is there, which is for whoever next re-checks the record.
//!
//! # What counts as a cited path
//!
//! A backticked token starting `crates/`, `docs/`, `plugins/` or `.github/`, with no
//! whitespace and no `*`. A token with a wildcard names a family of files, not one.

use std::fs;
use std::path::{Path, PathBuf};

const ROOTS: [&str; 4] = ["crates/", "docs/", "plugins/", ".github/"];

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/dpp-tests.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/dpp-tests sits two levels below the workspace root")
        .to_path_buf()
}

/// Every repository path `text` cites between backticks.
fn cited_paths(text: &str) -> Vec<String> {
    // The segments between an odd and the next even backtick are the code spans.
    text.split('`')
        .skip(1)
        .step_by(2)
        .filter(|span| ROOTS.iter().any(|root| span.starts_with(root)))
        .filter(|span| !span.contains(char::is_whitespace) && !span.contains('*'))
        .map(str::to_owned)
        .collect()
}

/// The paths `text` cites that do not exist under `root`.
fn missing(root: &Path, text: &str) -> Vec<String> {
    cited_paths(text)
        .into_iter()
        .filter(|path| !root.join(path).exists())
        .collect()
}

/// The files whose text is held to the rule: every record, and the prose beside them.
fn files() -> Vec<PathBuf> {
    let crate_dir = workspace_root().join("crates/dpp-vocab");
    let mut out: Vec<PathBuf> = fs::read_dir(crate_dir.join("vocabularies"))
        .expect("the vocabularies directory exists")
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "json" || e == "md"))
        .collect();
    out.push(crate_dir.join("README.md"));
    out.push(crate_dir.join("src/lib.rs"));
    out.sort();
    out
}

#[test]
fn every_repository_path_a_vocabulary_record_cites_exists() {
    let root = workspace_root();
    let mut gone = Vec::new();
    let mut checked = 0;
    for file in files() {
        let text = fs::read_to_string(&file).unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        checked += cited_paths(&text).len();
        for path in missing(&root, &text) {
            gone.push(format!(
                "{} cites {path}, which does not exist",
                file.file_name().and_then(|n| n.to_str()).unwrap_or("?")
            ));
        }
    }
    assert!(
        checked >= 5,
        "only {checked} cited paths were found, so the extraction has stopped working"
    );
    assert!(
        gone.is_empty(),
        "a record points at a file that moved or never existed. Update the path, and \
         read the claim again if the file changed:\n{}",
        gone.join("\n")
    );
}

#[test]
fn the_extraction_finds_a_path_and_ignores_what_is_not_one() {
    let text = "compared with `crates/dpp-domain/src/passport/record.rs` and `Passport::transition_to`, \
                see `docs/architecture/STANDARDS.md`, not `crates/*/src`, `docs/two words`, \
                `https://example.test/crates/x` or crates/unquoted.rs";
    assert_eq!(
        cited_paths(text),
        [
            "crates/dpp-domain/src/passport/record.rs",
            "docs/architecture/STANDARDS.md"
        ]
    );
}

#[test]
fn a_cited_path_that_does_not_exist_is_reported() {
    let root = workspace_root();
    let text = "`crates/does/not/exist.rs` and `crates/dpp-vocab/README.md`";
    assert_eq!(missing(&root, text), ["crates/does/not/exist.rs"]);
}
