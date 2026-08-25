//! Drift tripwire: no `mod.rs` in the workspace may declare a public item.
//!
//! A `mod.rs` is a pure index — module docs, `pub use` re-exports, and
//! submodule declarations only. Zero `pub struct` / `pub enum` / `pub trait` /
//! `pub fn` definitions. This keeps every `mod.rs` skimmable and forces new
//! types into their own named file as the crate grows.
//!
//! Rule 2 of `docs/architecture/CODE-LAYOUT.md`, and for a long time the only
//! rule there with a test behind it — which is why it is also the only one that
//! never drifted.

use std::fs;
use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Every `src` directory this rule governs, per `CODE-LAYOUT.md` §5: all
/// workspace crates and all Wasm plugins.
///
/// Discovered rather than listed, matching `layout.rs`. This test previously
/// carried a hand-written roster of eight crates, and `dpp-aas`, `dpp-vc` and
/// `dpp-vocab` — all three published — were never on it, so rule 2 had simply
/// never run against them. That is the exact failure the standard names: a
/// hardcoded roster is how a new crate ends up silently unchecked.
fn governed_src_dirs() -> Vec<PathBuf> {
    let root = workspace_root();
    let mut dirs = Vec::new();
    for group in ["crates", "plugins"] {
        let Ok(entries) = fs::read_dir(root.join(group)) else {
            continue;
        };
        for entry in entries.flatten() {
            let src = entry.path().join("src");
            if src.is_dir() {
                dirs.push(src);
            }
        }
    }
    assert!(
        dirs.len() > 10,
        "expected to discover the crates and plugins, found {} — has the repo layout moved?",
        dirs.len()
    );
    dirs
}

/// Recursively collect every `mod.rs` under `dir`.
fn find_mod_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_mod_rs_files(&path, out);
        } else if path.file_name().and_then(|n| n.to_str()) == Some("mod.rs") {
            out.push(path);
        }
    }
}

/// A line defines a public item if, once comments and doctest examples are
/// stripped, it starts with `pub struct` / `pub enum` / `pub trait` / `pub fn`
/// / `pub async fn`. `pub use` and `pub mod` are the allowed index content.
fn declares_public_item(line: &str) -> bool {
    let trimmed = line.trim();
    for prefix in [
        "pub struct ",
        "pub enum ",
        "pub trait ",
        "pub fn ",
        "pub async fn ",
    ] {
        if trimmed.starts_with(prefix) {
            return true;
        }
    }
    false
}

#[test]
fn mod_rs_files_are_pure_indexes() {
    let mut violations = Vec::new();

    for src_dir in governed_src_dirs() {
        let mut mod_files = Vec::new();
        find_mod_rs_files(&src_dir, &mut mod_files);

        for path in mod_files {
            let src = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            for (i, line) in src.lines().enumerate() {
                let trimmed = line.trim();
                // Every comment line goes, which drops the illustrative snippets
                // inside fenced doc examples with it — a `pub trait Foo` in an
                // example is not an item in this file. No fence tracking: see
                // `code_lines` in `layout.rs` for why a latching flag is worse
                // than none.
                if trimmed.starts_with("//") {
                    continue;
                }
                if declares_public_item(trimmed) {
                    violations.push(format!("{}:{}: {trimmed}", path.display(), i + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "\nmod.rs must be a pure index (module docs + `pub use` + `mod` decls only) — \
         move these items into their own file:\n{}\n",
        violations.join("\n")
    );
}
