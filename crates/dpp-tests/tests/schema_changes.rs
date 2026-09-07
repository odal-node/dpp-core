//! The generated record of what each schema version bump changed.
//!
//! `docs/architecture/SCHEMA-CHANGES.md` is a build artifact that is committed,
//! for the same reason `api/openapi.bundled.yaml` is committed in the engine: an
//! artifact reviewable in the diff that causes it gets read, and one that only
//! exists in a CI run does not.
//!
//! This test regenerates it and fails if the committed copy has drifted, so the
//! record cannot fall behind the schemas it describes. Regenerate with
//! `just schema-changes`.
//!
//! # Why generated and not written
//!
//! The CHANGELOG entry beside a schema bump is written by hand, which makes it a
//! restatement of a fact that lives in the JSON — and restatements drift. This
//! file cannot: it is derived from the schema files on every run, and the test
//! below is what makes that true rather than aspirational.
//!
//! # Why it lives here and not in `dpp-domain`
//!
//! It reads a path outside any crate — `docs/architecture/` belongs to the
//! repository, not to a package. `dpp-domain` is published, and `cargo package`
//! carries `tests/` along, so the same test vendored from crates.io would find
//! no report and fail claiming the schemas had drifted: a false accusation, in
//! a checkout that has nothing wrong with it. `dpp-tests` is `publish = false`
//! and already the home of the doc gates that read repository paths
//! (`domain_concerns.rs`, `ports_inventory.rs`).

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use dpp_domain::schemas::{ChangeKind, diff_schemas};
use semver::Version;

/// Environment variable that switches this test from checking to writing.
const WRITE_VAR: &str = "WRITE_SCHEMA_CHANGES";

fn manifest_relative(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn schemas_dir() -> PathBuf {
    manifest_relative("../dpp-domain/schemas")
}

fn report_path() -> PathBuf {
    manifest_relative("../../docs/architecture/SCHEMA-CHANGES.md")
}

/// Every product group's schema versions, semver-ordered.
fn schema_versions() -> BTreeMap<String, Vec<(Version, serde_json::Value)>> {
    let mut out: BTreeMap<String, Vec<(Version, serde_json::Value)>> = BTreeMap::new();

    for entry in fs::read_dir(schemas_dir()).expect("schemas dir") {
        let dir = entry.expect("dir entry").path();
        if !dir.is_dir() {
            continue;
        }
        let group = dir
            .file_name()
            .and_then(|s| s.to_str())
            .expect("product group name")
            .to_owned();

        let mut versions = Vec::new();
        for file in fs::read_dir(&dir).expect("product group dir") {
            let path = file.expect("file entry").path();
            let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let Some(raw) = name.strip_prefix('v').and_then(|n| n.strip_suffix(".json")) else {
                continue;
            };
            let version = Version::parse(raw)
                .unwrap_or_else(|e| panic!("{group}/{name} is not a semver filename: {e}"));
            let doc: serde_json::Value =
                serde_json::from_str(&fs::read_to_string(&path).expect("read schema"))
                    .unwrap_or_else(|e| panic!("{group}/{name} is not valid JSON: {e}"));
            versions.push((version, doc));
        }
        versions.sort_by(|a, b| a.0.cmp(&b.0));
        out.insert(group, versions);
    }
    out
}

fn render() -> String {
    let mut md = String::new();
    md.push_str(
        "# Schema changes\n\n\
         **Generated — do not edit.** Regenerate with `just schema-changes`;\n\
         `cargo test -p dpp-tests --test schema_changes` fails if this file has\n\
         drifted from the schemas under `crates/dpp-domain/schemas/`.\n\n\
         One section per product group, one table per version bump: what each\n\
         version changed relative to the one before it.\n\n\
         `additive` means the bump only adds properties — no removal, no type\n\
         change, no altered constraint, no newly required property. It is a\n\
         statement about *shape* and not a compatibility verdict; whether stored\n\
         documents still read is answered by the frozen fixtures in\n\
         `schema_compat.rs`, which test it rather than infer it.\n",
    );

    for (group, versions) in schema_versions() {
        md.push_str(&format!("\n## {group}\n\n"));

        if versions.len() < 2 {
            let only = versions
                .first()
                .map_or_else(|| "none".to_owned(), |(v, _)| format!("v{v}"));
            md.push_str(&format!(
                "Only one version ({only}) — nothing to compare against yet.\n"
            ));
            continue;
        }

        for pair in versions.windows(2) {
            let (prev_v, prev_doc) = &pair[0];
            let (next_v, next_doc) = &pair[1];
            let diff = diff_schemas(prev_doc, next_doc);

            md.push_str(&format!(
                "\n### v{prev_v} → v{next_v}{}\n\n",
                if diff.is_purely_additive() {
                    " · additive"
                } else {
                    ""
                }
            ));

            if diff.is_empty() {
                md.push_str("No property or requirement changed.\n");
                continue;
            }

            if !diff.properties.is_empty() {
                md.push_str("| Property | Change | Before | After |\n");
                md.push_str("|---|---|---|---|\n");
                for c in &diff.properties {
                    md.push_str(&format!(
                        "| `{}` | {} | {} | {} |\n",
                        c.path,
                        c.kind.label(),
                        cell(&c.before),
                        cell(&c.after),
                    ));
                }
            }

            if !diff.required_added.is_empty() {
                md.push_str(&format!(
                    "\n**Newly required:** {}\n",
                    diff.required_added
                        .iter()
                        .map(|r| format!("`{r}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !diff.required_removed.is_empty() {
                md.push_str(&format!(
                    "\n**No longer required:** {}\n",
                    diff.required_removed
                        .iter()
                        .map(|r| format!("`{r}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
    }
    md
}

/// Render a before/after cell: em dash for absent, escaped for the table.
fn cell(value: &str) -> String {
    if value.is_empty() {
        "—".to_owned()
    } else {
        format!("`{}`", value.replace('|', "\\|"))
    }
}

#[test]
fn the_committed_schema_change_report_matches_the_schemas() {
    let generated = render();
    let path = report_path();

    if std::env::var(WRITE_VAR).is_ok() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create docs dir");
        }
        fs::write(&path, &generated).expect("write report");
        return;
    }

    let committed = fs::read_to_string(&path).unwrap_or_default();
    // Compare line-normalised so a checkout with CRLF endings does not fail a
    // check about schema content.
    let norm = |s: &str| s.replace("\r\n", "\n");
    assert_eq!(
        norm(&committed),
        norm(&generated),
        "\n{} has drifted from the schemas it describes.\n\
         Regenerate it with `just schema-changes` and commit the result.\n",
        path.display()
    );
}

/// The report is only worth having if it actually reports. A run that produced
/// no tables would pass the drift check against an equally empty file.
#[test]
fn the_report_covers_every_product_group_with_more_than_one_version() {
    let md = render();
    let versions = schema_versions();

    let multi: Vec<&String> = versions
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(g, _)| g)
        .collect();
    assert!(
        !multi.is_empty(),
        "no product group has two schema versions — this test has stopped \
         proving anything and needs rewriting"
    );

    for group in multi {
        let versions = &versions[group];
        for pair in versions.windows(2) {
            let heading = format!("### v{} → v{}", pair[0].0, pair[1].0);
            assert!(
                md.contains(&heading),
                "{group} is missing the section for {heading}"
            );
        }
    }
}

/// The classification the report exists to make visible, asserted against real
/// schemas rather than fixtures: battery v2.1.0 → v2.2.0 added `stateOfHealth`.
#[test]
fn a_real_bump_is_classified_from_the_shipped_schemas() {
    let versions = schema_versions();
    let battery = &versions["battery"];
    let find = |want: &str| {
        battery
            .iter()
            .find(|(v, _)| v.to_string() == want)
            .unwrap_or_else(|| panic!("battery v{want} is missing"))
    };

    let diff = diff_schemas(&find("2.1.0").1, &find("2.2.0").1);
    assert!(
        diff.properties
            .iter()
            .any(|c| c.path == "stateOfHealth" && c.kind == ChangeKind::Added),
        "expected stateOfHealth to be reported as an addition, got {:?}",
        diff.properties
    );
}
