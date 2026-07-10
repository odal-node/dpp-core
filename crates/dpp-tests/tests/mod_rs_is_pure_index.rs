//! Drift tripwire: no `mod.rs` in a published crate may declare a public item.
//!
//! Mechanical enforcement of the re-layout's rule 2 (`docs/refactor-2026-07/core/00-INDEX.md`):
//! a `mod.rs` is a pure index — module docs, `pub use` re-exports, and
//! submodule declarations only. Zero `pub struct` / `pub enum` / `pub trait` /
//! `pub fn` definitions. This keeps every `mod.rs` skimmable and forces new
//! types into their own named file as the crate grows.

use std::fs;
use std::path::{Path, PathBuf};

const PUBLISHED_CRATES: &[&str] = &[
    "dpp-domain",
    "dpp-crypto",
    "dpp-digital-link",
    "dpp-plugin-traits",
    "dpp-plugin-sdk",
    "dpp-registry",
    "dpp-rules",
    "dpp-calc",
];

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
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
    let root = workspace_root();
    let mut violations = Vec::new();

    for krate in PUBLISHED_CRATES {
        let src_dir = root.join("crates").join(krate).join("src");
        let mut mod_files = Vec::new();
        find_mod_rs_files(&src_dir, &mut mod_files);

        for path in mod_files {
            let src = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
            let mut in_doctest = false;
            for (i, line) in src.lines().enumerate() {
                let trimmed = line.trim();
                // Skip fenced code blocks inside `///`/`//!` doc comments —
                // illustrative snippets (e.g. "pub trait Foo" in an example)
                // aren't real items in this file.
                if trimmed.starts_with("///") || trimmed.starts_with("//!") {
                    let doc_text = trimmed.trim_start_matches("///").trim_start_matches("//!");
                    if doc_text.trim_start().starts_with("```") {
                        in_doctest = !in_doctest;
                    }
                    continue;
                }
                if in_doctest || trimmed.starts_with("//") {
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
