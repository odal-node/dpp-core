//! Tripwire: a plugin's `validate_input` checks every field its schema makes
//! top-level `required`.
//!
//! # The two checks disagreed, and the plugin is the one that decides
//!
//! Schema validation runs separately at publish, so a plugin missing a check
//! is not an open door. It is worse in a quieter way: the plugin declares
//! itself **satisfied** with input that its own calculation then reads as
//! absent. `battery` did exactly that with `batteryType` — the field that
//! selects which obligations apply — and read it back as `unwrap_or("")`,
//! computing a determination against no instrument at all.
//!
//! Two gaps were found by hand, both pre-existing and neither introduced by the
//! work that found them:
//!
//! - `battery` never checked `batteryType`.
//! - `textile` never checked `productIdentifier` — the one plugin that never
//!   called `require_gtin`, so when every other plugin's call had to be
//!   replaced it had nothing to replace, and came through clean by accident.
//!
//! A third was found only by writing this test: the unsold-goods branch checked
//! neither `entity` nor `financialYear`, so a disclosure naming no discloser and
//! no reporting period passed.
//!
//! # What this can and cannot see
//!
//! It reads `require_*` / `optional_*` field-name literals out of the plugin
//! source, because `plugins/*` are excluded from the workspace and this crate
//! cannot call `validate_input`.
//!
//! 🚨 **Top-level only.** The SDK's `present` is `input.get(key)` with no path
//! traversal, so no `require_*` can reach a field inside a nested object; the
//! current schemas carry 38 nested `required` blocks and not one of them is
//! checkable at this tier. The single nested field required unconditionally
//! everywhere — `productIdentifier.scheme` — is the exception, because
//! `require_product_identifier` walks into the object itself.
//!
//! 🚨 It also cannot see **which branch** a call sits in, which matters for the
//! one crate serving two product groups. That is pinned behaviourally instead,
//! by `an_unsold_goods_report_still_needs_no_identifier` in the textile crate.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

/// Plugin crate → the product groups whose schemas it must satisfy.
///
/// One entry per crate. `product-group-textile` serves **two**: it dispatches
/// on the in-payload `productGroup` discriminant, because `meta().product_group`
/// can only name one and the host cannot select a dedicated unsold-goods
/// plugin. So `unsold-goods` has no directory of its own and its required
/// fields are this crate's to check.
const PLUGIN_GROUPS: &[(&str, &[&str])] = &[
    ("product-group-aluminium", &["aluminium"]),
    ("product-group-battery", &["battery"]),
    ("product-group-construction", &["construction"]),
    ("product-group-detergent", &["detergent"]),
    ("product-group-electronics", &["electronics"]),
    ("product-group-furniture", &["furniture"]),
    ("product-group-steel", &["steel"]),
    ("product-group-textile", &["textile", "unsold-goods"]),
    ("product-group-toy", &["toy"]),
    ("product-group-tyre", &["tyre"]),
];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("crates/dpp-tests sits two levels below the workspace root")
        .to_path_buf()
}

/// Every field name passed to a `require_*` or `optional_*` call in `src`.
fn validated_fields(src: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for (idx, _) in src
        .match_indices(".require_")
        .chain(src.match_indices(".optional_"))
    {
        let rest = &src[idx..];
        let Some(open) = rest.find('(') else { continue };
        // `.require_enum("batteryType", &BATTERY_TYPES)` — the field is the
        // first argument, and a method name never contains a quote.
        let head = &rest[..open];
        if head.contains('\n') || head.contains('"') {
            continue;
        }
        let after = &rest[open + 1..];
        let trimmed = after.trim_start();
        let Some(body) = trimmed.strip_prefix('"') else {
            continue;
        };
        if let Some(end) = body.find('"') {
            out.insert(body[..end].to_owned());
        }
    }
    out
}

/// The top-level `required` list of a product group's current schema.
fn required_fields(root: &Path, group: &str) -> Vec<String> {
    let dir = root.join("crates/dpp-domain/schemas").join(group);
    let mut versions: Vec<(Vec<u64>, PathBuf)> = fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("reading {}: {e}", dir.display()))
        .flatten()
        .filter_map(|e| {
            let p = e.path();
            let name = p.file_name()?.to_str()?;
            let core = name.strip_prefix('v')?.strip_suffix(".json")?;
            let parts: Option<Vec<u64>> = core.split('.').map(|s| s.parse().ok()).collect();
            Some((parts?, p))
        })
        .collect();
    versions.sort();
    let (_, newest) = versions.last().expect("a product group ships some schema");
    let text = fs::read_to_string(newest).expect("schema is readable");
    let doc: serde_json::Value = serde_json::from_str(&text).expect("schema is JSON");
    doc.get("required")
        .and_then(|r| r.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn every_plugin_checks_every_field_its_schema_requires() {
    let root = workspace_root();
    let mut problems: Vec<String> = Vec::new();
    let mut checked = 0usize;

    for (crate_name, groups) in PLUGIN_GROUPS {
        let dir = root.join("plugins").join(crate_name);
        assert!(
            dir.is_dir(),
            "{crate_name} is in PLUGIN_GROUPS but not on disk — update the list deliberately"
        );

        // The whole crate, not just lib.rs: a branch may live in a submodule.
        let mut sources = Vec::new();
        collect_rs(&dir.join("src"), &mut sources);
        let joined: String = sources
            .iter()
            .filter_map(|p| fs::read_to_string(p).ok())
            .collect::<Vec<_>>()
            .join("\n");
        let validated = validated_fields(&joined);

        for group in *groups {
            checked += 1;
            for field in required_fields(&root, group) {
                if !validated.contains(&field) {
                    problems.push(format!(
                        "{crate_name}: '{group}' schema requires '{field}', validate_input never checks it"
                    ));
                }
            }
        }
    }

    assert!(
        problems.is_empty(),
        "\nA plugin declares itself satisfied with input its own schema would \
         refuse:\n\n  {}\n\n\
         Schema validation still runs at publish, so this is not an open door \
         — it is the plugin and the schema disagreeing, with the plugin being \
         the one that produces the compliance determination.\n",
        problems.join("\n  ")
    );

    // 🚨 A count, because a loop over an empty list passes. Ten crates, eleven
    // product groups: the textile crate answers for two.
    assert_eq!(
        checked, 11,
        "expected 11 (crate, product group) pairs, checked {checked}"
    );
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn the_plugin_list_covers_every_plugin_on_disk() {
    // Otherwise a new plugin is simply absent from the table above and the test
    // reports nothing about it — the vacuous pass this file exists to prevent,
    // one level up.
    let root = workspace_root();
    let on_disk: BTreeSet<String> = fs::read_dir(root.join("plugins"))
        .expect("plugins/ is readable")
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let n = e.file_name().to_str()?.to_owned();
            n.starts_with("product-group-").then_some(n)
        })
        .collect();
    let listed: BTreeSet<String> = PLUGIN_GROUPS
        .iter()
        .map(|(name, _)| (*name).to_owned())
        .collect();

    assert!(!on_disk.is_empty(), "found no plugins — the path is wrong");
    assert_eq!(
        on_disk, listed,
        "PLUGIN_GROUPS and plugins/ disagree; a new plugin needs the product \
         groups it answers for written down"
    );
}
