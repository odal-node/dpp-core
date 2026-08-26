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
            // An inline test module is `mod <name> {`; the sibling-file form is
            // `mod <name>;`, which is what the rule asks for.
            //
            // Any test-ish name counts, not just `tests`. An earlier version
            // matched `mod tests {` alone and so missed five modules named for
            // their subject — `mod passport_wire_keys_tests {` and friends — one
            // of which then fooled a measurement into reporting a violation that
            // was only ever a fixture.
            let inline = code_lines(&src).iter().any(|l| {
                let Some(rest) = l
                    .strip_prefix("mod ")
                    .or_else(|| l.strip_prefix("pub mod "))
                else {
                    return false;
                };
                if !rest.ends_with('{') {
                    return false;
                }
                let name = rest.trim_end_matches('{').trim();
                name == "tests" || name.ends_with("_tests")
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

// ---------------------------------------------------------------------------
// Rule 0 — imports point up the tier ladder
// ---------------------------------------------------------------------------

/// The tier of each top-level module in `dpp-domain`, per `CODE-LAYOUT.md` §1.
///
/// Only `dpp-domain` carries tiers today. The law is stated for this crate
/// because it is the one that grew a cycle, and adding a module to this table is
/// a deliberate act — an unlisted module is not silently exempt, it fails.
const TIERS: &[(&str, u8)] = &[
    ("identifier", 1),
    ("catalog", 2),
    ("compliance", 2),
    ("schemas", 2),
    ("error", 2),
    ("field_error", 2),
    ("eol", 2),
    ("facility", 2),
    ("graph", 2),
    ("credential", 2),
    ("disclosure", 2),
    ("instrument", 2),
    ("manufacturer", 2),
    ("material", 2),
    ("passport", 2),
    ("product", 2),
    ("product_group", 2),
    ("seal", 2),
    ("status", 2),
    ("transfer", 2),
    ("access", 3),
    ("lint", 3),
    ("validation", 3),
    ("passthrough", 4),
    ("ports", 4),
];

fn tier_of(module: &str) -> Option<u8> {
    TIERS.iter().find(|(m, _)| *m == module).map(|(_, t)| *t)
}

/// Test files may reach anywhere: a test exercises the thing it covers from
/// outside, and the ladder is a statement about production dependencies.
fn is_test_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    name == "tests.rs"
        || name == "golden_vectors.rs"
        || name.ends_with("_tests.rs")
        || path.components().any(|c| c.as_os_str() == "tests")
}

#[test]
fn rule_0_tier_imports_point_up() {
    let root = workspace_root().join("crates/dpp-domain/src");

    // An unlisted module used to be skipped, which made the table's own doc
    // ("an unlisted module is not silently exempt, it fails") untrue — a new
    // top-level module would have been exempt from the ladder until somebody
    // noticed. Placing a module in a tier is the decision this rule exists to
    // force, so not deciding has to fail.
    let mut unlisted: Vec<String> = Vec::new();
    if let Ok(entries) = fs::read_dir(&root) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if tier_of(name).is_none() {
                unlisted.push(name.to_owned());
            }
        }
    }
    assert!(
        unlisted.is_empty(),
        "\nCODE-LAYOUT.md rule 0 — these modules are not in the tier table:\n{}\n\n\
         Add each to `TIERS` with the tier it belongs in. A module with no tier \
         is not exempt from the ladder; it is undecided.\n",
        unlisted
            .iter()
            .map(|m| format!("  {m}"))
            .collect::<Vec<_>>()
            .join("\n")
    );

    let mut files = Vec::new();
    find_rs_files(&root, &mut files);

    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for path in files {
        if is_test_file(&path) {
            continue;
        }
        let Some(src) = read_source(&path) else {
            continue;
        };
        if has_deviation_marker(&src) {
            continue;
        }
        // The file's own module is the first path component under `src/`.
        let relative = rel(&path);
        let Some(rest) = relative.strip_prefix("crates/dpp-domain/src/") else {
            continue;
        };
        let Some((own, _)) = rest.split_once('/') else {
            continue; // lib.rs and friends sit above the ladder
        };
        let Some(own_tier) = tier_of(own) else {
            continue;
        };

        let mut breaches: BTreeSet<String> = BTreeSet::new();
        for line in code_lines(&src) {
            let mut rest = line;
            while let Some(at) = rest.find("crate::") {
                rest = &rest[at + "crate::".len()..];
                let target: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if target == own {
                    continue;
                }
                if let Some(t) = tier_of(&target)
                    && t > own_tier
                {
                    breaches.insert(format!("{target}(t{t})"));
                }
            }
        }
        if !breaches.is_empty() {
            found.insert(
                relative,
                format!(
                    "t{own_tier} imports {}",
                    breaches.into_iter().collect::<Vec<_>>().join(", ")
                ),
            );
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 0 — imports may only point up the tier ladder",
        &found,
        TIER_BASELINE,
    );
}

// Empty. The ladder holds with no exceptions. Two things got it there: the
// `error` cycle was broken by splitting `field_error` out, and `schemas` moved
// to tier 2 — where its imports always put it — rather than tier 3, where a
// "policy" label had put it by hand.
const TIER_BASELINE: &[&str] = &[];

// ---------------------------------------------------------------------------
// Rule 11 — a concept that outgrew its file becomes a directory
// ---------------------------------------------------------------------------

/// Proxy, in the same sense rule 1's is. A test cannot tell whether two files
/// are one concept; it can tell that `gtin.rs` sits beside `gtin_check_digit.rs`,
/// which is what an outgrown concept looks like from outside.
#[test]
fn rule_11_an_outgrown_concept_is_a_directory() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in governed_src_dirs() {
        let mut dirs = vec![dir];
        while let Some(d) = dirs.pop() {
            let Ok(entries) = fs::read_dir(&d) else {
                continue;
            };
            let mut stems: Vec<(String, PathBuf)> = Vec::new();
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    dirs.push(path);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs")
                    && let Some(stem) = path.file_stem().and_then(|s| s.to_str())
                {
                    stems.push((stem.to_owned(), path.clone()));
                }
            }
            for (stem, path) in &stems {
                for (other, _) in &stems {
                    if other == stem || !other.starts_with(&format!("{stem}_")) {
                        continue;
                    }
                    // Any `*_tests.rs` beside it is the sibling-test file
                    // rules 4 and 7 require. Flagging it would make two rules
                    // contradict each other, and the one that fires first wins.
                    if other.ends_with("_tests") {
                        continue;
                    }
                    let Some(src) = read_source(path) else {
                        continue;
                    };
                    if has_deviation_marker(&src) {
                        continue;
                    }
                    found.insert(rel(path), format!("{other}.rs is part of it"));
                }
            }
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 11 — a concept spanning two files becomes a directory",
        &found,
        OUTGROWN_CONCEPT_BASELINE,
    );
}

// Empty since 2026-08-26: `catalog/instrument*.rs` became the `instrument/`
// module, which is exactly what this rule was asking for.
const OUTGROWN_CONCEPT_BASELINE: &[&str] = &[];

// ---------------------------------------------------------------------------
// Rule 12 — a file never repeats the name of its directory
// ---------------------------------------------------------------------------

#[test]
fn rule_12_no_name_repeats_its_directory() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in governed_src_dirs() {
        let mut files = Vec::new();
        find_rs_files(&dir, &mut files);
        for path in files {
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
                continue;
            };
            let Some(parent) = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
            else {
                continue;
            };
            // `ghosts/ghost_archive.rs` repeats just as surely as
            // `battery/battery_data.rs`, so a plural directory is compared in
            // its singular form too.
            //
            // An exact match — `passport/passport.rs` — is the purest form of the
            // same fault, and the one clippy already reports as
            // `module_inception`. Six files here carried an `#[allow]` for it,
            // which is what a rule looks like when it is silenced rather than
            // kept.
            let singular = parent.strip_suffix('s').unwrap_or(parent);
            let repeats = stem == parent
                || stem == singular
                || stem.starts_with(&format!("{parent}_"))
                || stem.starts_with(&format!("{singular}_"));
            if !repeats {
                continue;
            }
            let Some(src) = read_source(&path) else {
                continue;
            };
            if has_deviation_marker(&src) {
                continue;
            }
            found.insert(rel(&path), format!("inside {parent}/"));
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 12 — a file never repeats the name of its directory",
        &found,
        NAME_REPEATS_BASELINE,
    );
}

// `dpp-vocab` is another crate and its own pass. `dpp-domain` cleared this rule
// on 2026-08-26 when `gtin/` became the `identifier/` leaf.
const NAME_REPEATS_BASELINE: &[&str] = &["crates/dpp-vocab/src/register/register.rs"];

// ---------------------------------------------------------------------------
// Rule 13 — hyphens outside module paths, underscores inside
// ---------------------------------------------------------------------------

/// A directory under `src/` is a Rust identifier, and identifiers cannot contain
/// a hyphen — so the split is a language constraint rather than a preference.
/// Everything that is *not* a module path takes the hyphen: crate directories,
/// plugin directories, and the data directories beside them.
#[test]
fn rule_13_hyphens_outside_module_paths() {
    let root = workspace_root();
    let mut found: BTreeMap<String, String> = BTreeMap::new();

    for dir in governed_src_dirs() {
        let mut files = Vec::new();
        find_rs_files(&dir, &mut files);
        for path in files {
            let relative = rel(&path);
            let Some((_, after_src)) = relative.split_once("/src/") else {
                continue;
            };
            if after_src.contains('-') {
                found.insert(relative, "hyphen in a module path".to_owned());
            }
        }
    }

    for group in ["crates", "plugins"] {
        let Ok(entries) = fs::read_dir(root.join(group)) else {
            continue;
        };
        for entry in entries.flatten() {
            let krate = entry.path();
            if !krate.is_dir() {
                continue;
            }
            let name = krate.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name.contains('_') {
                found.insert(rel(&krate), "underscore in a crate directory".to_owned());
            }
            let Ok(inner) = fs::read_dir(&krate) else {
                continue;
            };
            for sub in inner.flatten() {
                let path = sub.path();
                if !path.is_dir() {
                    continue;
                }
                let sub_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if matches!(
                    sub_name,
                    "src" | "tests" | "benches" | "examples" | "target"
                ) {
                    continue;
                }
                if sub_name.contains('_') {
                    found.insert(rel(&path), "underscore in a data directory".to_owned());
                }
            }
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 13 — hyphens outside module paths, underscores inside",
        &found,
        HYPHEN_BASELINE,
    );
}

const HYPHEN_BASELINE: &[&str] = &[];

// ---------------------------------------------------------------------------
// Rule 14 — an error lives in an error.rs
// ---------------------------------------------------------------------------

#[test]
fn rule_14_errors_live_in_error_rs() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in governed_src_dirs() {
        let mut files = Vec::new();
        find_rs_files(&dir, &mut files);
        for path in files {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // An `error/` directory satisfies the rule exactly as an `error.rs`
            // does — the point is that a module's failure modes sit in one known
            // place, not that the place is a single file. A crate-wide error
            // surface outgrows one file and rule 11 then requires the directory,
            // so demanding the file here would put two rules in contradiction.
            let in_error_dir = path
                .parent()
                .and_then(|p| p.file_name())
                .and_then(|n| n.to_str())
                == Some("error");
            if name == "error.rs" || in_error_dir || is_test_file(&path) {
                continue;
            }
            let Some(src) = read_source(&path) else {
                continue;
            };
            if has_deviation_marker(&src) {
                continue;
            }
            let errors: Vec<String> = code_lines(&src)
                .iter()
                .filter_map(|l| {
                    let rest = l
                        .strip_prefix("pub enum ")
                        .or_else(|| l.strip_prefix("pub struct "))?;
                    let ident = rest.split(|c: char| !c.is_alphanumeric()).next()?;
                    ident.ends_with("Error").then(|| ident.to_owned())
                })
                .collect();
            if !errors.is_empty() {
                found.insert(rel(&path), errors.join(", "));
            }
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 14 — an error type lives in an error.rs",
        &found,
        ERROR_PLACEMENT_BASELINE,
    );
}

const ERROR_PLACEMENT_BASELINE: &[&str] = &[
    "crates/dpp-aas/src/builder.rs",
    "crates/dpp-crypto/src/jades/header.rs",
    "crates/dpp-domain/src/schemas/lens/transform.rs",
    "crates/dpp-domain/src/schemas/lens/upcast_error.rs",
    "crates/dpp-domain/src/schemas/registration_error.rs",
    "crates/dpp-rules/src/bundle/types.rs",
];

// ---------------------------------------------------------------------------
// Rule 15 — a shared thing lives at the nearest common parent of its users
// ---------------------------------------------------------------------------

/// Directory names that assert "several of my siblings use these".
///
/// A roster, unlike the crate discovery elsewhere in this file, because the claim
/// is *semantic*: no directory listing reveals that a module is meant to be
/// shared. `enums` was on it until that bucket was dissolved on 2026-08-26 —
/// nine enums, of which one was actually shared.
const SHARED_BUCKETS: &[&str] = &["common", "shared"];

#[test]
fn rule_15_shared_means_shared() {
    let mut found: BTreeMap<String, String> = BTreeMap::new();
    for dir in governed_src_dirs() {
        let mut dirs = vec![dir];
        while let Some(d) = dirs.pop() {
            let Ok(entries) = fs::read_dir(&d) else {
                continue;
            };
            let children: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
            for path in &children {
                if path.is_dir() {
                    dirs.push(path.clone());
                }
            }
            for bucket in &children {
                let name = bucket.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !bucket.is_dir() || !SHARED_BUCKETS.contains(&name) {
                    continue;
                }
                let mut bucket_files = Vec::new();
                find_rs_files(bucket, &mut bucket_files);

                // Each sibling of the bucket is one candidate user, and it counts
                // once however many times it mentions the type. Counting textual
                // occurrences instead would let a single sibling that imports a
                // type and then names it in one field look like two users, which
                // is exactly the false "shared" this rule exists to catch.
                //
                // Tests do not count: a fixture reaching for a type is not a
                // sibling depending on it. Neither does a `mod.rs` — re-exporting
                // a type is indexing it, not using it, and counting the index
                // would let every bucket in the tree look shared by one hop.
                let mut sibling_srcs: Vec<String> = Vec::new();
                for sibling in &children {
                    if sibling == bucket {
                        continue;
                    }
                    let mut sfiles = Vec::new();
                    if sibling.is_dir() {
                        find_rs_files(sibling, &mut sfiles);
                    } else {
                        sfiles.push(sibling.clone());
                    }
                    let mut joined = String::new();
                    for f in sfiles {
                        if is_test_file(&f)
                            || f.file_name().and_then(|n| n.to_str()) == Some("mod.rs")
                        {
                            continue;
                        }
                        if let Some(s) = read_source(&f) {
                            joined.push_str(&s);
                        }
                    }
                    sibling_srcs.push(joined);
                }

                for file in bucket_files {
                    if is_test_file(&file) {
                        continue;
                    }
                    let Some(src) = read_source(&file) else {
                        continue;
                    };
                    if has_deviation_marker(&src) {
                        continue;
                    }
                    let lonely: Vec<String> = code_lines(&src)
                        .iter()
                        .filter_map(|l| {
                            let rest = l
                                .strip_prefix("pub enum ")
                                .or_else(|| l.strip_prefix("pub struct "))
                                .or_else(|| l.strip_prefix("pub trait "))?;
                            let ident = rest.split(|c: char| !c.is_alphanumeric()).next()?;
                            let uses = sibling_srcs.iter().filter(|s| s.contains(ident)).count();
                            (uses < 2).then(|| format!("{ident} used by {uses}"))
                        })
                        .collect();
                    if !lonely.is_empty() {
                        found.insert(rel(&file), lonely.join(", "));
                    }
                }
            }
        }
    }
    assert_against_baseline(
        "CODE-LAYOUT.md rule 15 — a type in a shared bucket needs at least two users",
        &found,
        SHARED_BUCKET_BASELINE,
    );
}

const SHARED_BUCKET_BASELINE: &[&str] = &["crates/dpp-rules/src/common/date.rs"];

/// Every production import edge between two top-level `dpp-domain` modules.
///
/// Test files are excluded on purpose: a test reaches for whatever it needs to
/// exercise the thing it covers, and the ladder is a statement about production
/// dependencies. Including them would report cycles that do not exist at
/// runtime — measured on 2026-08-26, two of the three apparent cycles in this
/// crate were `tests.rs` files.
fn module_edges() -> BTreeSet<(String, String)> {
    let root = workspace_root().join("crates/dpp-domain/src");
    let mut files = Vec::new();
    find_rs_files(&root, &mut files);

    let mut edges = BTreeSet::new();
    for path in files {
        if is_test_file(&path) {
            continue;
        }
        let Some(src) = read_source(&path) else {
            continue;
        };
        let relative = rel(&path);
        let Some(rest) = relative.strip_prefix("crates/dpp-domain/src/") else {
            continue;
        };
        let Some((own, _)) = rest.split_once('/') else {
            continue;
        };
        if tier_of(own).is_none() {
            continue;
        }
        for line in code_lines(&src) {
            let mut rest = line;
            while let Some(at) = rest.find("crate::") {
                rest = &rest[at + "crate::".len()..];
                let target: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                if target != own && tier_of(&target).is_some() {
                    edges.insert((own.to_owned(), target));
                }
            }
        }
    }
    edges
}

#[test]
fn rule_0_module_graph_is_acyclic() {
    let edges = module_edges();
    let mut remaining: BTreeSet<String> = TIERS.iter().map(|(m, _)| (*m).to_owned()).collect();

    // Kahn, from the leaves up: drop any module that no longer depends on
    // anything still in the set, and repeat. Whatever cannot be dropped is in a
    // cycle, or depends on one.
    loop {
        let leaves: Vec<String> = remaining
            .iter()
            .filter(|m| {
                !edges
                    .iter()
                    .any(|(src, dst)| src == *m && remaining.contains(dst))
            })
            .cloned()
            .collect();
        if leaves.is_empty() {
            break;
        }
        for leaf in leaves {
            remaining.remove(&leaf);
        }
    }

    if remaining.is_empty() {
        return;
    }

    let mut detail = String::new();
    for m in &remaining {
        let deps: Vec<&str> = edges
            .iter()
            .filter(|(src, dst)| src == m && remaining.contains(dst))
            .map(|(_, dst)| dst.as_str())
            .collect();
        detail.push_str(&format!("  {m} -> {}\n", deps.join(", ")));
    }
    panic!(
        "\nCODE-LAYOUT.md rule 0 — these modules form an import cycle:\n\n{detail}\n\
         A cycle passes the tier check whenever both modules sit in the same tier, \
         so direction alone does not catch it. It usually means one module is two \
         things at different levels — `error` was, until `field_error` was split \
         out of it on 2026-08-26.\n"
    );
}
