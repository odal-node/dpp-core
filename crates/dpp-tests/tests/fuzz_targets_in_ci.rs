//! Drift tripwire: every fuzz target `fuzz/Cargo.toml` builds is run by the
//! nightly fuzz job, and the job runs nothing the manifest does not build.
//!
//! # Why this needs a test rather than care
//!
//! The fuzz crate is its own workspace and builds only on nightly, so the stable
//! gate never compiles it, and its targets run only through the matrix in
//! `.github/workflows/fuzz.yml`. A target missing from that list never runs, and
//! nothing else says so: `plugin_abi_envelope` sat in the manifest without once
//! being run, until the list was checked by hand.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/dpp-tests.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/dpp-tests sits two levels below the workspace root")
        .to_path_buf()
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The `name` of every `[[bin]]` table in a Cargo manifest.
fn manifest_targets(manifest: &str) -> BTreeSet<String> {
    let mut targets = BTreeSet::new();
    let mut in_bin = false;
    for line in manifest.lines().map(str::trim) {
        if line.starts_with('[') {
            in_bin = line == "[[bin]]";
            continue;
        }
        if !in_bin {
            continue;
        }
        if let Some(value) = line
            .strip_prefix("name")
            .and_then(|rest| rest.trim_start().strip_prefix('='))
        {
            targets.insert(value.trim().trim_matches('"').to_owned());
        }
    }
    targets
}

/// The entries of the `target:` list in a workflow's matrix, comments skipped.
fn workflow_targets(workflow: &str) -> BTreeSet<String> {
    let mut targets = BTreeSet::new();
    let mut lines = workflow.lines().skip_while(|line| line.trim() != "target:");
    lines.next();
    for line in lines.map(str::trim) {
        if line.starts_with('#') {
            continue;
        }
        match line.strip_prefix("- ") {
            Some(name) => {
                targets.insert(name.trim().to_owned());
            }
            None => break,
        }
    }
    targets
}

#[test]
fn every_fuzz_target_runs_in_the_nightly_job() {
    let root = workspace_root();
    let built = manifest_targets(&read(&root.join("fuzz/Cargo.toml")));
    let run = workflow_targets(&read(&root.join(".github/workflows/fuzz.yml")));

    // Two empty sets are equal. Without these, a format change that defeats
    // either parser would pass silently.
    assert!(!built.is_empty(), "no [[bin]] read from fuzz/Cargo.toml");
    assert!(
        !run.is_empty(),
        "no matrix `target:` list read from fuzz.yml"
    );

    let not_run: Vec<_> = built.difference(&run).collect();
    let not_built: Vec<_> = run.difference(&built).collect();
    assert!(
        not_run.is_empty() && not_built.is_empty(),
        "fuzz targets built but never run: {not_run:?}; run but not built: {not_built:?}. \
         Add each `[[bin]]` in fuzz/Cargo.toml to the matrix in .github/workflows/fuzz.yml"
    );
}

#[test]
fn the_parsers_read_bins_and_matrix_entries_and_nothing_else() {
    let manifest = r#"
[package]
name = "not-a-target"

[[bin]]
name = "first"
path = "fuzz_targets/first.rs"

[dependencies]
name = "also-not-a-target"

[[bin]]
name    = "second"
"#;
    let expected: BTreeSet<String> = ["first", "second"].map(String::from).into();
    assert_eq!(manifest_targets(manifest), expected);

    let workflow = r"
    strategy:
      matrix:
        target:
          # a comment between entries
          - first
          - second
    steps:
      - uses: actions/checkout
";
    assert_eq!(workflow_targets(workflow), expected);
}
