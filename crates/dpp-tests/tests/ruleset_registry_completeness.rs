//! Drift tripwire: every `impl Ruleset` in `dpp-calc` reaches `all_rulesets()`.
//!
//! `ruleset_registry::resolve::all_rulesets()` is a hand-maintained list, and
//! two safety properties are asserted by iterating it:
//!
//! - `an_unsourced_ruleset_may_not_claim_its_numbers_are_law` — the tripwire
//!   that keeps the `ParameterBasis::Sourced` default honest. The default is
//!   deliberately fail-closed, so a ruleset that says nothing reads as carrying
//!   law, and that test is the only thing stopping a *new* placeholder from
//!   acquiring legal provenance by staying silent.
//! - `every_ruleset_declares_the_numbers_it_computes_with` — the check that no
//!   ruleset declares an empty parameter set, which would make its
//!   `ruleset_content_sha256` the hash of `{}` and identical to every other
//!   silent ruleset's.
//!
//! Both iterate the list, so both stop applying to a ruleset nobody registers —
//! silently, and without the diff looking wrong, because the diff that should
//! have carried the row is the diff that didn't. The only pre-existing gate on
//! the list ran the other way: `active_map_entries_reference_real_rulesets`
//! checks that every calculator-map entry names a ruleset that exists. Nothing
//! checked that every ruleset *reaches* the list.
//!
//! This is a source-text gate rather than a compile-time one because Rust cannot
//! enumerate a trait's implementors on stable. The alternatives are a registry
//! macro (a new dependency, plus link-section tricks that do not play well with
//! the `wasm32` targets this workspace builds for) or a proc-macro attribute on
//! every impl — which is the same "remember to add it" problem wearing a
//! different hat.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

fn manifest_relative(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

// ---------------------------------------------------------------------------
// Side 1 — the impls in the source
// ---------------------------------------------------------------------------

/// Recursively collect every `.rs` file under `dir`.
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

/// Files declared by a parent module as `#[cfg(test)] mod <name>;`.
///
/// These are compiled only under `cfg(test)`, so the local test doubles in them
/// (`SaysNothing`, and the deliberately unregistered stubs the provenance tests
/// build) are not rulesets this crate ships and must not be required to appear
/// in the registry.
///
/// Only whole modules gated at their declaration are exempt. An `impl Ruleset`
/// inside an *inline* `#[cfg(test)] mod tests { … }` block in a shipping file
/// is still reported: telling those apart needs a latching flag over the file,
/// and `layout.rs` records why that is worse than none. Moving such a double
/// into a gated module file is the fix, and the failure message says so.
fn test_gated_files(src_dir: &Path) -> BTreeSet<PathBuf> {
    let mut gated = BTreeSet::new();
    let mut index_files = Vec::new();
    find_rs_files(src_dir, &mut index_files);

    for path in index_files {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if name != "mod.rs" && name != "lib.rs" {
            continue;
        }
        let Ok(src) = fs::read_to_string(&path) else {
            continue;
        };
        let dir = path.parent().unwrap_or(src_dir);
        let lines: Vec<&str> = src.lines().map(str::trim).collect();
        for (i, line) in lines.iter().enumerate() {
            if *line != "#[cfg(test)]" {
                continue;
            }
            let Some(next) = lines.get(i + 1) else {
                continue;
            };
            let Some(module) = next.strip_prefix("mod ").and_then(|m| m.strip_suffix(';')) else {
                continue;
            };
            let module = module.trim();
            gated.insert(dir.join(format!("{module}.rs")));
            gated.insert(dir.join(module).join("mod.rs"));
        }
    }
    gated
}

/// What one `impl … Ruleset for …` line names.
enum ImplTarget {
    /// A concrete type that must appear in `all_rulesets()`.
    Concrete(String),
    /// A type parameterised only by lifetimes — `FilledRepairabilityRuleset<'a>`
    /// and anything like it.
    ///
    /// Exempt, and the exemption is structural rather than a name on a list: a
    /// row in `&'static [&'static dyn Ruleset]` has to be a `'static` value, and
    /// a type that borrows cannot be one. Such wrappers also delegate `id()`,
    /// `effectivity()`, `regulatory_basis()` and `parameter_basis()` to a base
    /// ruleset, so both provenance properties are already asserted against the
    /// base — which this gate *does* require to be registered.
    ///
    /// A type parameterised by anything else is **not** exempt: `Foo<Bar>` can
    /// perfectly well be a `'static` row, so it has to be registered like any
    /// other.
    BorrowedWrapper,
}

/// Parse an `impl` line, or `None` if it does not implement the base `Ruleset`
/// trait.
///
/// Matching on `" Ruleset for "` is what keeps the sub-traits out:
/// `impl RepairabilityRuleset for X` has no space before `Ruleset`, so it does
/// not match — and it does not need to, since every concrete ruleset here
/// implements the base trait directly as well.
fn ruleset_impl_target(line: &str) -> Option<Result<ImplTarget, String>> {
    let trimmed = line.trim();
    if !trimmed.starts_with("impl") {
        return None;
    }
    let rest = trimmed.split_once(" Ruleset for ")?.1.trim_start();

    let ident_len = rest
        .find(|c: char| !(c.is_alphanumeric() || c == '_'))
        .unwrap_or(rest.len());
    if ident_len == 0 {
        return Some(Err(format!("could not read a type name out of: {trimmed}")));
    }
    let (ident, tail) = rest.split_at(ident_len);

    let Some(generics) = tail.trim_start().strip_prefix('<') else {
        return Some(Ok(ImplTarget::Concrete(ident.to_owned())));
    };
    let Some(close) = generics.find('>') else {
        return Some(Err(format!(
            "unterminated generic list, cannot classify: {trimmed}"
        )));
    };
    // Lifetimes only, or it is an ordinary concrete row that happens to be
    // generic. Failing towards `Concrete` is the fail-closed direction: the
    // worst case is a registry row someone has to add.
    let lifetimes_only = generics[..close]
        .split(',')
        .all(|param| param.trim().starts_with('\''));
    Some(Ok(if lifetimes_only {
        ImplTarget::BorrowedWrapper
    } else {
        ImplTarget::Concrete(ident.to_owned())
    }))
}

/// Every concrete type in `dpp-calc` that implements `Ruleset`, with the
/// unparseable impl lines reported separately.
fn implemented_rulesets(src_dir: &Path) -> (BTreeSet<String>, Vec<String>) {
    let gated = test_gated_files(src_dir);
    let mut files = Vec::new();
    find_rs_files(src_dir, &mut files);

    let mut found = BTreeSet::new();
    let mut unparsed = Vec::new();

    for path in files {
        if gated.contains(&path) {
            continue;
        }
        let src =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
        for (i, line) in src.lines().enumerate() {
            // Comment lines go first. This crate's doc comments carry a lot of
            // illustrative Rust, and an `impl Ruleset for …` inside a fenced
            // example is not an impl in this file — the same reason
            // `mod_rs_is_pure_index` and `layout.rs` rule 7 both strip them.
            if line.trim().starts_with("//") {
                continue;
            }
            match ruleset_impl_target(line) {
                None => {}
                Some(Ok(ImplTarget::Concrete(name))) => {
                    found.insert(name);
                }
                Some(Ok(ImplTarget::BorrowedWrapper)) => {}
                Some(Err(why)) => {
                    unparsed.push(format!("{}:{}: {why}", path.display(), i + 1));
                }
            }
        }
    }
    (found, unparsed)
}

// ---------------------------------------------------------------------------
// Side 2 — the registry
// ---------------------------------------------------------------------------

/// Type names listed as rows in `all_rulesets()`.
fn registered_rulesets() -> BTreeSet<String> {
    let path = manifest_relative("../dpp-calc/src/ruleset_registry/resolve.rs");
    let src = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()));

    let start = src
        .find("pub fn all_rulesets()")
        .expect("resolve.rs no longer defines `pub fn all_rulesets()` — has it moved?");
    let body = &src[start..];
    // The function's own closing brace, at column zero.
    let end = body
        .find("\n}")
        .expect("could not find the end of `all_rulesets()`");

    body[..end]
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("//"))
        .filter_map(|line| line.strip_prefix('&')?.strip_suffix(','))
        .filter(|name| {
            !name.is_empty()
                && name.chars().all(|c| c.is_alphanumeric() || c == '_')
                && !name.starts_with(|c: char| c.is_ascii_digit())
        })
        .map(str::to_owned)
        .collect()
}

// ---------------------------------------------------------------------------
// The gate
// ---------------------------------------------------------------------------

#[test]
fn every_ruleset_impl_reaches_the_registry() {
    let src_dir = manifest_relative("../dpp-calc/src");
    let (implemented, unparsed) = implemented_rulesets(&src_dir);
    let registered = registered_rulesets();

    assert!(
        unparsed.is_empty(),
        "\nthis gate could not classify these `impl Ruleset` lines, so it cannot \
         vouch for them — reshape them or teach `ruleset_impl_target` about the \
         form:\n{}\n",
        unparsed.join("\n")
    );

    // A parser that quietly stops matching would let this test pass while
    // checking nothing, which is the same silent failure it exists to prevent.
    assert!(
        implemented.len() > 5,
        "found only {} `impl Ruleset` in {} — the scanner is broken, not the \
         registry",
        implemented.len(),
        src_dir.display()
    );

    let missing: Vec<&String> = implemented.difference(&registered).collect();
    let stale: Vec<&String> = registered.difference(&implemented).collect();

    assert!(
        missing.is_empty() && stale.is_empty(),
        "\nruleset registry drift — `all_rulesets()` in \
         dpp-calc/src/ruleset_registry/resolve.rs must list every ruleset.\n\
         \n\
         implements `Ruleset` but is not registered: {missing:?}\n\
         registered but no impl was found:           {stale:?}\n\
         \n\
         An unregistered ruleset escapes both provenance tripwires: it takes the \
         fail-closed `ParameterBasis::Sourced` default *and* misses \
         `an_unsourced_ruleset_may_not_claim_its_numbers_are_law`, and it misses \
         `every_ruleset_declares_the_numbers_it_computes_with` too, so it can \
         issue receipts whose parameter hash attests to nothing.\n\
         \n\
         Add a row to `all_rulesets()`. If the impl is a test double, move it \
         into a module its parent declares `#[cfg(test)] mod <name>;` — an \
         inline `#[cfg(test)]` block in a shipping file is not exempt.\n\
         \n\
         A name under `registered but no impl was found` more likely means this \
         gate's scanner has drifted than that the row is wrong.\n"
    );
}
