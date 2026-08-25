//! Drift tripwires for `docs/architecture/CODE-LAYOUT.md`.
//!
//! One test per enforced rule. Rule 2 lives in its own file
//! (`mod_rs_is_pure_index.rs`) because it predates this one and works; the rest
//! are here, sharing one directory walk.
//!
//! # How these fail
//!
//! Each test carries a `BASELINE`: the files that already violated the rule when
//! it landed. Those are allowed. **Anything not on the list fails.** So the rules
//! bind for all new and moved code from day one while the backlog is worked
//! through separately, and a shrinking baseline is the visible measure of that.
//!
//! **Never add to a baseline to go green.** That is what
//! `// LAYOUT-DEVIATION: <reason>` is for — it is greppable and it has to state a
//! reason, where a baseline entry states nothing.
//!
//! A stale baseline entry is also a failure: a file that has been fixed must be
//! removed from the list, or the list slowly stops describing anything.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

/// Marker that exempts a file from an enforced layout rule.
const DEVIATION_MARKER: &str = "LAYOUT-DEVIATION:";

/// Rule 4's threshold, in lines.
const MAX_TESTS_FILE_LINES: usize = 400;

/// Rule 1's proxy: a file with this many public types has stopped being about
/// one thing. Three rather than two because a type and its own error enum are
/// one concept and should not need a marker.
const MAX_PUBLIC_TYPES: usize = 3;

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Every `src` directory this standard governs: all workspace crates and all
/// Wasm plugins.
///
/// Discovered rather than listed. A hardcoded roster is how a new crate ends up
/// silently unchecked, which is the same failure mode these tests exist to
/// prevent.
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

fn find_rs_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_rs_files(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// Repo-relative, forward-slashed, so a baseline entry reads the same on every
/// platform and in every diff.
fn rel(path: &Path) -> String {
    let root = workspace_root();
    let root = root.canonicalize().unwrap_or(root);
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    path.strip_prefix(&root)
        .unwrap_or(&path)
        .to_string_lossy()
        .replace('\\', "/")
}

fn has_deviation_marker(src: &str) -> bool {
    src.contains(DEVIATION_MARKER)
}

/// Read a source file with any UTF-8 byte-order mark stripped.
///
/// U+FEFF is not whitespace, so `trim_start` leaves it in place and a file an
/// editor saved with a BOM reads as `\u{feff}//! …`. Rule 8 then reports it as
/// having no module doc when line 1 plainly is one. Two files in this workspace
/// were baselined on exactly that misreading, and on Windows a BOM is one
/// careless "save as" away — so it is stripped once, here, for every rule.
fn read_source(path: &Path) -> Option<String> {
    let src = fs::read_to_string(path).ok()?;
    Some(match src.strip_prefix('\u{feff}') {
        Some(stripped) => stripped.to_owned(),
        None => src,
    })
}

/// Compare found violations against the baseline and report both directions.
///
/// Fails on a new violation *and* on a stale baseline entry, because a list that
/// is never pruned stops being a record of anything.
///
/// `found` maps a repo-relative path to a human detail ("892 lines"). **Only the
/// path is matched against the baseline.** Putting the measurement in the key
/// would make every baselined file fail the moment anyone touched it for an
/// unrelated reason, and a tripwire that fires on innocent edits is one that
/// gets deleted.
fn assert_against_baseline(rule: &str, found: &BTreeMap<String, String>, baseline: &[&str]) {
    let baseline: BTreeSet<String> = baseline.iter().map(|s| (*s).to_owned()).collect();
    let found_paths: BTreeSet<String> = found.keys().cloned().collect();

    let new: Vec<&String> = found_paths.difference(&baseline).collect();
    let fixed: Vec<&String> = baseline.difference(&found_paths).collect();

    let mut message = String::new();
    if !new.is_empty() {
        message.push_str(&format!(
            "\n{rule}\n\nThese files break the rule and are not in the baseline:\n"
        ));
        for v in &new {
            let detail = found.get(*v).map(String::as_str).unwrap_or("");
            if detail.is_empty() {
                message.push_str(&format!("  {v}\n"));
            } else {
                message.push_str(&format!("  {v}  ({detail})\n"));
            }
        }
        message.push_str(
            "\nFix the file, or mark it `// LAYOUT-DEVIATION: <reason>`. \
             Do not add it to the baseline.\n",
        );
    }
    if !fixed.is_empty() {
        message.push_str(&format!(
            "\n{rule}\n\nThese are in the baseline but no longer violate it — \
             remove them from the baseline:\n"
        ));
        for v in &fixed {
            message.push_str(&format!("  {v}\n"));
        }
    }
    assert!(message.is_empty(), "{message}");
}

/// Strip every comment line, so an illustrative `pub struct Foo` inside a doc
/// example is not counted as an item.
///
/// Shared by the rule 1 and rule 7 scanners. Mirrors the same handling in
/// `mod_rs_is_pure_index.rs`, which needs it for the same reason: this crate's
/// doc comments contain a lot of illustrative Rust.
///
/// No fence tracking, deliberately. Every line inside a ```` ``` ```` block in a
/// doc comment is itself a `///` or `//!` line, so dropping comment lines
/// already drops the examples. An earlier version carried an `in_doctest` flag
/// that could never be observed on a code line — but an odd number of fences
/// anywhere in a file latched it `true` and silently swallowed every remaining
/// line, which would have taken rules 1 and 7 off duty for that file with
/// nothing going red.
fn code_lines(src: &str) -> Vec<&str> {
    src.lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .collect()
}

// ---------------------------------------------------------------------------
// Rule 1 + 5 — one public type per file, when the type has gravity
// ---------------------------------------------------------------------------

/// Files that already declared three or more public types when this landed.
const ONE_TYPE_PER_FILE_BASELINE: &[&str] = &[
    "crates/dpp-aas/src/model.rs",
    "crates/dpp-calc/src/co2e/calculator.rs",
    "crates/dpp-calc/src/co2e/cfb.rs",
    "crates/dpp-calc/src/kernel/ruleset.rs",
    "crates/dpp-calc/src/recycled_content/thresholds.rs",
    "crates/dpp-calc/src/repairability/calculator.rs",
    "crates/dpp-calc/src/repairability_index/thresholds.rs",
    "crates/dpp-calc/src/ruleset_registry/status.rs",
    "crates/dpp-crypto/src/jades/header.rs",
    "crates/dpp-crypto/src/keystore/store.rs",
    "crates/dpp-plugin-traits/src/meta.rs",
    "crates/dpp-plugin-traits/src/result.rs",
    "crates/dpp-plugin-traits/src/version.rs",
    "crates/dpp-registry/src/error.rs",
    "crates/dpp-registry/src/identifiers.rs",
    "crates/dpp-registry/src/response.rs",
    "crates/dpp-rules/src/batteries/recycled_content.rs",
    "crates/dpp-rules/src/bundle/types.rs",
    "crates/dpp-rules/src/chemicals/svhc.rs",
    "crates/dpp-vc/src/credential/trust.rs",
    "crates/dpp-vc/src/credential/types.rs",
];

#[test]
fn rule_1_one_public_type_per_file() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in governed_src_dirs() {
        let mut files = Vec::new();
        find_rs_files(&dir, &mut files);
        for path in files {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // `mod.rs` is rule 2's problem; `lib.rs` legitimately re-exports;
            // a tests file's fixtures are not the module's public surface.
            if matches!(name, "mod.rs" | "lib.rs" | "tests.rs" | "golden_vectors.rs") {
                continue;
            }
            let Some(src) = read_source(&path) else {
                continue;
            };
            if has_deviation_marker(&src) {
                continue;
            }
            let count = code_lines(&src)
                .iter()
                .filter(|l| {
                    l.starts_with("pub struct ")
                        || l.starts_with("pub enum ")
                        || l.starts_with("pub trait ")
                })
                .count();
            if count >= MAX_PUBLIC_TYPES {
                found.insert(rel(&path), format!("{count} public types"));
            }
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 1 — one public type per file, when the type has gravity",
        &found,
        ONE_TYPE_PER_FILE_BASELINE,
    );
}

// ---------------------------------------------------------------------------
// Rule 4 — a tests file splits when it passes 400 lines
// ---------------------------------------------------------------------------

const TESTS_FILE_SIZE_BASELINE: &[&str] = &[
    "crates/dpp-aas/src/tests.rs",
    "crates/dpp-crypto/src/jades/tests.rs",
    "crates/dpp-crypto/src/keystore/tests.rs",
    "crates/dpp-digital-link/src/digital_link/tests.rs",
    "crates/dpp-registry/src/tests.rs",
    "crates/dpp-vc/src/credential/tests.rs",
    "crates/dpp-vc/src/tests.rs",
];

#[test]
fn rule_4_tests_files_are_navigable() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in governed_src_dirs() {
        let mut files = Vec::new();
        find_rs_files(&dir, &mut files);
        for path in files {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if !matches!(name, "tests.rs" | "golden_vectors.rs") && !name.ends_with("_tests.rs") {
                continue;
            }
            let Some(src) = read_source(&path) else {
                continue;
            };
            if has_deviation_marker(&src) {
                continue;
            }
            let lines = src.lines().count();
            if lines > MAX_TESTS_FILE_LINES {
                found.insert(rel(&path), format!("{lines} lines"));
            }
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 4 — a tests file splits when it passes 400 lines",
        &found,
        TESTS_FILE_SIZE_BASELINE,
    );
}

// ---------------------------------------------------------------------------
// Rule 6 + 9 — only lib.rs at src root, plus test_support.rs
// ---------------------------------------------------------------------------

const ROOT_FILES_BASELINE: &[&str] = &[
    "crates/dpp-aas/src/builder.rs",
    "crates/dpp-aas/src/mapper.rs",
    "crates/dpp-aas/src/model.rs",
    "crates/dpp-aas/src/property.rs",
    "crates/dpp-aas/src/templates.rs",
    "crates/dpp-aas/src/tests.rs",
    "crates/dpp-plugin-sdk/src/abi.rs",
    "crates/dpp-plugin-sdk/src/codec.rs",
    "crates/dpp-plugin-sdk/src/entry.rs",
    "crates/dpp-plugin-sdk/src/tests.rs",
    "crates/dpp-plugin-sdk/src/validate.rs",
    "crates/dpp-plugin-traits/src/error.rs",
    "crates/dpp-plugin-traits/src/meta.rs",
    "crates/dpp-plugin-traits/src/plugin.rs",
    "crates/dpp-plugin-traits/src/result.rs",
    "crates/dpp-plugin-traits/src/tests.rs",
    "crates/dpp-plugin-traits/src/version.rs",
    "crates/dpp-registry/src/endpoint.rs",
    "crates/dpp-registry/src/error.rs",
    "crates/dpp-registry/src/granularity.rs",
    "crates/dpp-registry/src/identifiers.rs",
    "crates/dpp-registry/src/payload.rs",
    "crates/dpp-registry/src/response.rs",
    "crates/dpp-registry/src/tests.rs",
    "crates/dpp-registry/src/transfer.rs",
    // Rule 9 names this `test_support.rs`; renaming it is a later phase.
    "crates/dpp-tests/src/fixtures.rs",
    "crates/dpp-vc/src/did_builder.rs",
    "crates/dpp-vc/src/local_service.rs",
    "crates/dpp-vc/src/passport_credential.rs",
    "crates/dpp-vc/src/status_list.rs",
    "crates/dpp-vc/src/tests.rs",
    "plugins/product-group-textile/src/fibre_composition.rs",
    "plugins/product-group-textile/src/unsold_goods.rs",
];

#[test]
fn rule_6_only_lib_rs_at_src_root() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in governed_src_dirs() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() || path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Rule 9 makes `test_support.rs` the one permitted exception.
            if matches!(name, "lib.rs" | "main.rs" | "test_support.rs") {
                continue;
            }
            let src = read_source(&path).unwrap_or_default();
            if has_deviation_marker(&src) {
                continue;
            }
            found.insert(rel(&path), String::new());
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 6 — only lib.rs at a crate's src root (rule 9 allows test_support.rs)",
        &found,
        ROOT_FILES_BASELINE,
    );
}

// ---------------------------------------------------------------------------
// Rule 7 — tests live in a sibling file, never inline
// ---------------------------------------------------------------------------

const INLINE_TESTS_BASELINE: &[&str] = &[
    "crates/dpp-calc/src/co2e/calculator.rs",
    "crates/dpp-calc/src/co2e/cfb.rs",
    "crates/dpp-calc/src/co2e/gwp_factors.rs",
    "crates/dpp-calc/src/kernel/assessability.rs",
    "crates/dpp-calc/src/kernel/clock.rs",
    "crates/dpp-calc/src/kernel/receipt.rs",
    "crates/dpp-calc/src/kernel/synthetic_factor.rs",
    "crates/dpp-calc/src/repairability/calculator.rs",
    "crates/dpp-crypto/src/jws/canonical.rs",
    "crates/dpp-digital-link/src/digital_link/codec.rs",
    "crates/dpp-digital-link/src/digital_link/element_string.rs",
    "crates/dpp-digital-link/src/digital_link/qr.rs",
    "crates/dpp-digital-link/src/digital_link/syntax_dictionary.rs",
    "crates/dpp-digital-link/src/linktype/media_type.rs",
    "crates/dpp-digital-link/src/linktype/vocabulary.rs",
    "crates/dpp-plugin-sdk/src/validate.rs",
    "crates/dpp-registry/src/granularity.rs",
    "crates/dpp-rules/src/batteries/chemistry.rs",
    "crates/dpp-rules/src/batteries/degradation.rs",
    "crates/dpp-rules/src/batteries/passport_content.rs",
    "crates/dpp-rules/src/batteries/recycled_content.rs",
    "crates/dpp-rules/src/chemicals/cas.rs",
    "crates/dpp-rules/src/chemicals/surfactants.rs",
    "crates/dpp-rules/src/chemicals/svhc.rs",
    "crates/dpp-rules/src/common/country.rs",
    "crates/dpp-rules/src/common/date.rs",
    "crates/dpp-rules/src/common/numeric.rs",
    "crates/dpp-rules/src/lint/battery.rs",
    "crates/dpp-rules/src/lint/textile.rs",
    "crates/dpp-rules/src/metals/aluminium.rs",
    "crates/dpp-rules/src/textiles/fibre.rs",
    "crates/dpp-rules/src/unsold_goods/annex_vii.rs",
    "crates/dpp-vc/src/local_service.rs",
    "crates/dpp-vc/src/passport_credential.rs",
    "crates/dpp-vc/src/status_list.rs",
    // Every plugin keeps its tests inline; the plugin pass is a later phase.
    "plugins/product-group-aluminium/src/lib.rs",
    "plugins/product-group-battery/src/lib.rs",
    "plugins/product-group-construction/src/lib.rs",
    "plugins/product-group-detergent/src/lib.rs",
    "plugins/product-group-electronics/src/lib.rs",
    "plugins/product-group-furniture/src/lib.rs",
    "plugins/product-group-steel/src/lib.rs",
    "plugins/product-group-textile/src/lib.rs",
    "plugins/product-group-toy/src/lib.rs",
    "plugins/product-group-tyre/src/lib.rs",
];

#[test]
fn rule_7_tests_are_siblings_not_inline() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in governed_src_dirs() {
        let mut files = Vec::new();
        find_rs_files(&dir, &mut files);
        for path in files {
            let Some(src) = read_source(&path) else {
                continue;
            };
            if has_deviation_marker(&src) {
                continue;
            }
            // An inline test module is `mod tests {`; the sibling-file form is
            // `mod tests;`, which is what the rule asks for.
            let inline = code_lines(&src).iter().any(|l| {
                (l.starts_with("mod tests") || l.starts_with("pub mod tests")) && l.ends_with('{')
            });
            if inline {
                found.insert(rel(&path), String::new());
            }
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 7 — tests live in a sibling tests.rs, never inline",
        &found,
        INLINE_TESTS_BASELINE,
    );
}

// ---------------------------------------------------------------------------
// Rule 8 — every file opens with a module doc
// ---------------------------------------------------------------------------

const MODULE_DOCS_BASELINE: &[&str] = &[
    "crates/dpp-aas/src/mapper.rs",
    "crates/dpp-aas/src/model.rs",
    "crates/dpp-aas/src/product_groups/battery.rs",
    "crates/dpp-aas/src/product_groups/electronics.rs",
    "crates/dpp-aas/src/product_groups/textile.rs",
    "crates/dpp-aas/src/property.rs",
    "crates/dpp-aas/src/semantic_ids/mod.rs",
    "crates/dpp-aas/src/templates.rs",
    "crates/dpp-aas/src/tests.rs",
    "crates/dpp-crypto/src/jws/tests.rs",
    "crates/dpp-crypto/src/keystore/migration.rs",
    "crates/dpp-crypto/src/keystore/rotation.rs",
    "crates/dpp-crypto/src/keystore/tests.rs",
    "crates/dpp-rules/src/canonical/hash.rs",
    "crates/dpp-rules/src/canonical/tests.rs",
    "crates/dpp-vc/src/credential/builder.rs",
    "crates/dpp-vc/src/credential/revocation.rs",
    "crates/dpp-vc/src/credential/tests.rs",
    "crates/dpp-vc/src/credential/trust.rs",
    "crates/dpp-vc/src/credential/types.rs",
    "crates/dpp-vc/src/credential/verify.rs",
    "crates/dpp-vc/src/tests.rs",
    "crates/dpp-vocab/src/register/tests.rs",
];

#[test]
fn rule_8_every_file_has_module_docs() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in governed_src_dirs() {
        let mut files = Vec::new();
        find_rs_files(&dir, &mut files);
        for path in files {
            let Some(src) = read_source(&path) else {
                continue;
            };
            if has_deviation_marker(&src) {
                continue;
            }
            let has_doc = src
                .lines()
                .take(3)
                .any(|l| l.trim_start().starts_with("//!"));
            if !has_doc {
                found.insert(rel(&path), String::new());
            }
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 8 — every file opens with a `//!` module doc",
        &found,
        MODULE_DOCS_BASELINE,
    );
}
