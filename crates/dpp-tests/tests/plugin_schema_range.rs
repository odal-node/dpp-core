//! Tripwire: a plugin's declared `schema_version_range` follows the schemas
//! its product group actually ships.
//!
//! # Why this needs a test rather than care
//!
//! Adding `crates/dpp-domain/schemas/<group>/vX.Y.Z.json` is deliberately a
//! single file addition — the registry picks it up by `include_str!` and
//! nothing else has to change. Nothing asks whether the plugin for that group
//! claims to accept it, so the two drift apart silently and in one direction:
//! the schema moves, the plugin does not.
//!
//! 🚨 **All ten were stale at once.** Before #322 every plugin's `max_version`
//! was behind its group — aluminium 1.1.0 against 1.2.0, battery 2.6.0 against
//! 2.7.0, electronics 1.2.0 against 1.4.0, furniture 1.1.0 against 1.3.0, and
//! so on for all ten. They were corrected by hand, which is the same mechanism
//! that let them drift, so the correction has the same shelf life as the last
//! one unless something holds it.
//!
//! # Why a wrong value is currently invisible
//!
//! The range is read by nothing at dispatch: the host calls
//! `check_compatibility(…, None, …)` and a `None` schema version skips the
//! check entirely. That is precisely why ten wrong bounds across ten crates
//! went unnoticed — a value nothing reads cannot fail.
//!
//! So this is a trap rather than a live defect, and it is worth closing *while*
//! it is one. The day dispatch-time schema enforcement is switched on, ten
//! stale bounds become ten wrong answers, and which way each is wrong decides
//! whether it refuses a valid passport or accepts one it cannot handle.
//!
//! # Why the source is read as text
//!
//! `plugins/*` are excluded from the workspace — `exclude = ["plugins/*"]` —
//! so this crate cannot call `schema_version_range()`. It reads the literal out
//! of the source instead, the same way `gtin_enforcement.rs` reads payload
//! field declarations.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Product groups that ship schemas and have no plugin, each for a stated
/// reason. Asserted as an exact set so a thirteenth group cannot appear
/// unplugged without someone deciding that it should.
///
/// - `unsold-goods` is a disclosure over a reporting period rather than a
///   product, and has no compliance determination to make.
/// - `mattress` has no plugin yet. Recorded rather than fixed here: writing one
///   is not this test's business, but its absence should be a decision.
const GROUPS_WITHOUT_A_PLUGIN: &[&str] = &["mattress", "unsold-goods"];

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/dpp-tests.
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/dpp-tests sits two levels below the workspace root")
        .to_path_buf()
}

/// `v1.10.0` sorts above `v1.9.0`, which a string comparison gets wrong — and
/// gets wrong silently, by picking a real version that is not the highest.
fn parse_version(name: &str) -> Option<(u64, u64, u64)> {
    let core = name.strip_prefix('v')?;
    // `unwrap_or(core)`, not `unwrap_or(name)` — the fallback has to be the
    // string with the `v` already removed, or a bare `v1.2.0` parses as `v1`.
    let core = core.strip_suffix(".json").unwrap_or(core);
    let mut parts = core.split('.');
    let out = (
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
    );
    parts.next().is_none().then_some(out)
}

/// The string literal assigned to `field` inside `schema_version_range`.
///
/// Scoped to that function so a `min_version` mentioned in a doc comment or
/// another method cannot be read as the declaration.
fn declared_bound(src: &str, field: &str) -> Option<String> {
    let body = src.split_once("fn schema_version_range")?.1;
    let line = body
        .lines()
        .take_while(|l| !l.trim_start().starts_with("fn "))
        .find(|l| l.trim_start().starts_with(&format!("{field}:")))?;
    let (_, rest) = line.split_once('"')?;
    let (value, _) = rest.split_once('"')?;
    Some(value.to_owned())
}

/// Every version a product group ships, lowest first.
fn shipped_versions(root: &Path, group: &str) -> Vec<(u64, u64, u64)> {
    let dir = root.join("crates/dpp-domain/schemas").join(group);
    let Ok(entries) = fs::read_dir(&dir) else {
        return Vec::new();
    };
    let mut out: Vec<(u64, u64, u64)> = entries
        .flatten()
        .filter_map(|e| parse_version(e.file_name().to_str()?))
        .collect();
    out.sort_unstable();
    out
}

fn fmt(v: (u64, u64, u64)) -> String {
    format!("{}.{}.{}", v.0, v.1, v.2)
}

#[test]
fn every_plugin_declares_the_schema_versions_its_group_ships() {
    let root = workspace_root();
    let plugins_dir = root.join("plugins");

    let mut checked = 0usize;
    let mut problems: Vec<String> = Vec::new();

    let entries = fs::read_dir(&plugins_dir).expect("plugins/ must be readable");
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_dir()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("product-group-"))
        })
        .collect();
    dirs.sort();

    for dir in &dirs {
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        let group = name.trim_start_matches("product-group-");
        let Ok(src) = fs::read_to_string(dir.join("src/lib.rs")) else {
            problems.push(format!("{name}: src/lib.rs is unreadable"));
            continue;
        };
        checked += 1;

        let shipped = shipped_versions(&root, group);
        if shipped.is_empty() {
            // The other half of the same question: a plugin naming a product
            // group that ships no schema at all.
            problems.push(format!(
                "{name}: names product group '{group}', which ships no schemas"
            ));
            continue;
        }
        let highest = *shipped.last().expect("non-empty");

        let (Some(min), Some(max)) = (
            declared_bound(&src, "min_version"),
            declared_bound(&src, "max_version"),
        ) else {
            problems.push(format!("{name}: no schema_version_range literals found"));
            continue;
        };

        match parse_version(&format!("v{max}")) {
            Some(declared) if declared == highest => {}
            Some(declared) => problems.push(format!(
                "{name}: declares max_version {} but '{group}' ships up to {}",
                fmt(declared),
                fmt(highest)
            )),
            None => problems.push(format!("{name}: max_version '{max}' is not a version")),
        }

        // The lower bound is not required to be the lowest version shipped — a
        // plugin may legitimately stop accepting an old shape, and nothing has
        // decided that it may not. It must name a version that exists, which is
        // what catches a typo.
        match parse_version(&format!("v{min}")) {
            Some(declared) if shipped.contains(&declared) => {}
            Some(declared) => problems.push(format!(
                "{name}: declares min_version {} which '{group}' does not ship",
                fmt(declared)
            )),
            None => problems.push(format!("{name}: min_version '{min}' is not a version")),
        }
    }

    assert!(
        problems.is_empty(),
        "\nA plugin's declared schema range disagrees with the schemas its \
         product group ships:\n\n  {}\n\n\
         Nothing reads this range at dispatch today — the host passes `None` \
         and the check is skipped — so a wrong value fails nothing until \
         schema enforcement is switched on, and then fails everything at \
         once.\n",
        problems.join("\n  ")
    );

    // 🚨 A count, because a loop over a directory that matches nothing passes.
    // These crates were once `plugins/sector-*`, and the rename silently
    // emptied a glob that had no such guard.
    assert_eq!(
        checked, 10,
        "expected 10 product-group plugins, read {checked} — if one was added \
         or removed, update this count deliberately"
    );
}

#[test]
fn a_product_group_without_a_plugin_is_one_someone_chose() {
    let root = workspace_root();

    let schema_groups: BTreeSet<String> = fs::read_dir(root.join("crates/dpp-domain/schemas"))
        .expect("the schema directory must be readable")
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().to_str().map(str::to_owned))
        .collect();

    let plugin_groups: BTreeSet<String> = fs::read_dir(root.join("plugins"))
        .expect("plugins/ must be readable")
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            e.file_name()
                .to_str()?
                .strip_prefix("product-group-")
                .map(str::to_owned)
        })
        .collect();

    assert!(
        !schema_groups.is_empty() && !plugin_groups.is_empty(),
        "read no product groups at all — the paths are wrong, not the tree"
    );

    let unplugged: Vec<&str> = schema_groups
        .difference(&plugin_groups)
        .map(String::as_str)
        .collect();
    let expected: Vec<&str> = GROUPS_WITHOUT_A_PLUGIN.to_vec();

    assert_eq!(
        unplugged, expected,
        "\nThe set of product groups shipping schemas with no plugin has \
         changed.\n\nA new one means a group whose compliance determination \
         nothing makes — the host has no strategy to dispatch to, and a \
         passport in that group is published without one. If that is intended, \
         add it to GROUPS_WITHOUT_A_PLUGIN with the reason.\n"
    );
}
