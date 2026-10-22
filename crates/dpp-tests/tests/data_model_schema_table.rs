//! Drift tripwire: the schema versions `docs/architecture/DATA-MODEL.md` states
//! must be the ones each product group's manifest declares current — the
//! "Current schema" column of the table in §4.3, and the versions in the headings
//! of the two sections above it.
//!
//! The column is written by hand, and it went a full version stale for eight of
//! its ten rows before anyone noticed: every product group gained a schema when
//! the product identifier stopped requiring a GTIN, and the table did not move.
//! The manifest under `dpp-domain/product-groups/` is the authority, so this
//! reads the table and checks it against the catalog, in both directions: a row
//! whose version is not the current one, and a product group that has no row.
//!
//! Battery and textile have a section of their own above the table instead of a
//! row, and the table says so. They are the only groups allowed to have no row,
//! and that is asserted rather than assumed, so a new product group cannot slip
//! out of the document unnoticed. The heading of each section states a version
//! too, and that is checked in the same way.
//!
//! **A failure on a heading is not fixed by changing the number.** A heading
//! claims that the section below it describes that version's shape, and this
//! test can only see the number. The textile heading read v1.2.0 while the schema
//! was at v1.4.0, and the section under it was missing a required field and nine
//! optional ones. When a heading fails, re-read the section against the type and
//! the schema, and then change it.
//!
//! Lives here and not in `dpp-domain` for the reason `schema_changes.rs` does:
//! it reads a path outside any crate, and `dpp-domain` is published.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use dpp_domain::catalog::ProductGroupCatalog;

/// Product groups documented in a section of their own above the table, with the
/// Rust type the section's heading names: `### 4.1 Battery Product group
/// (`BatteryData`) — v2.9.0`.
const GROUPS_WITH_THEIR_OWN_SECTION: &[(&str, &str)] =
    &[("battery", "BatteryData"), ("textile", "TextileData")];

/// The header line that opens the table. Matched whole, so a renamed column or a
/// moved table fails loudly instead of leaving the test with nothing to read.
const TABLE_HEADER: &str = "| Product group | Type | Current schema | Note |";

fn data_model() -> String {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/architecture/DATA-MODEL.md");
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
}

/// The (group, version) pair of each row of the table, as written.
fn table_rows(doc: &str) -> Vec<(String, String)> {
    let mut lines = doc.lines();
    assert!(
        lines.by_ref().any(|l| l.trim() == TABLE_HEADER),
        "DATA-MODEL.md has no table headed `{TABLE_HEADER}` — was it renamed or moved?"
    );
    lines
        .take_while(|l| l.trim_start().starts_with('|'))
        .filter_map(|l| {
            // `| group | type | version | note |` splits into an empty cell, four
            // cells and whatever the note adds; only the first three matter.
            let cells: Vec<&str> = l.split('|').map(str::trim).collect();
            let (group, version) = (cells.get(1)?, cells.get(3)?);
            // The `|---|---|` rule under the header is not a row.
            if group.chars().all(|c| matches!(c, '-' | ':' | ' ')) {
                return None;
            }
            Some((group.trim_matches('`').to_owned(), (*version).to_owned()))
        })
        .collect()
}

/// Everything wrong with `rows` against `current`, one sentence each.
fn drift(
    rows: &[(String, String)],
    current: &BTreeMap<String, String>,
    own_section: &[&str],
) -> Vec<String> {
    let mut problems = Vec::new();
    let mut seen: Vec<&str> = Vec::new();

    for (group, version) in rows {
        if seen.contains(&group.as_str()) {
            problems.push(format!("`{group}` has more than one row"));
        }
        seen.push(group);

        match current.get(group) {
            None => problems.push(format!("`{group}` has a row but is not in the catalog")),
            Some(declared) if *version != format!("v{declared}") => problems.push(format!(
                "`{group}`: the table says {version}, the manifest says v{declared}"
            )),
            Some(_) => {}
        }
        if own_section.contains(&group.as_str()) {
            problems.push(format!(
                "`{group}` is listed as having a section of its own, and has a row as well"
            ));
        }
    }

    for group in current.keys() {
        if !seen.contains(&group.as_str()) && !own_section.contains(&group.as_str()) {
            problems.push(format!(
                "`{group}` is in the catalog and has neither a row nor a section of its own"
            ));
        }
    }
    problems
}

/// Everything wrong with the version each own-section heading states, one
/// sentence each. A heading reads `### 4.1 Battery Product group
/// (`BatteryData`) — v2.9.0`; the type in backticks finds it and the text after
/// the dash is the version.
fn heading_drift(
    doc: &str,
    sections: &[(&str, &str)],
    current: &BTreeMap<String, String>,
) -> Vec<String> {
    let mut problems = Vec::new();

    for (group, type_name) in sections {
        let marker = format!("(`{type_name}`)");
        let headings: Vec<&str> = doc
            .lines()
            .filter(|l| l.starts_with("### ") && l.contains(&marker))
            .collect();
        let [heading] = headings.as_slice() else {
            problems.push(format!(
                "`{group}` should have exactly one section heading naming `{type_name}`, \
                 found {}",
                headings.len()
            ));
            continue;
        };

        let Some(declared) = current.get(*group) else {
            problems.push(format!("`{group}` has a section but is not in the catalog"));
            continue;
        };
        match heading.rsplit_once('—').map(|(_, stated)| stated.trim()) {
            None => problems.push(format!(
                "`{group}`: the heading states no version: {heading}"
            )),
            Some(stated) if stated != format!("v{declared}") => problems.push(format!(
                "`{group}`: the heading says {stated}, the manifest says v{declared} — \
                 re-read the section below it before changing the number"
            )),
            Some(_) => {}
        }
    }
    problems
}

#[test]
fn data_model_schema_versions_match_the_catalog() {
    let catalog = ProductGroupCatalog::new();
    let current: BTreeMap<String, String> = catalog
        .all()
        .iter()
        .map(|d| (d.key.clone(), d.current_schema_version.clone()))
        .collect();
    let doc = data_model();
    let rows = table_rows(&doc);

    // A parser that quietly stops matching would let this pass while checking
    // nothing, which is how a drift gate fails.
    assert!(
        rows.len() >= 5,
        "parsed {} rows from the DATA-MODEL.md table — the parser or the table has changed shape: {rows:?}",
        rows.len()
    );

    let own_section: Vec<&str> = GROUPS_WITH_THEIR_OWN_SECTION
        .iter()
        .map(|(group, _)| *group)
        .collect();
    let mut problems = drift(&rows, &current, &own_section);
    problems.extend(heading_drift(&doc, GROUPS_WITH_THEIR_OWN_SECTION, &current));
    assert!(
        problems.is_empty(),
        "\nthe schema versions in DATA-MODEL.md have drifted from the product group manifests. \
         Update the \"Current schema\" cells in §4.3; for a section heading, re-read the \
         section against the type and the schema before changing its number.\n  {}\n",
        problems.join("\n  ")
    );
}

/// The check is only worth having if it catches each way the table can be wrong,
/// and passes a table that is right.
#[test]
fn the_table_check_catches_what_it_is_for() {
    let current: BTreeMap<String, String> = [
        ("alpha", "1.2.0"),
        ("beta", "2.0.0"),
        ("gamma", "1.0.0"),
        ("delta", "3.1.0"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();

    let doc = "\
Some prose.

| Product group | Type | Current schema | Note |
|---|---|---|---|
| `alpha` | `AlphaData` | v1.1.0 | stale: the manifest moved |
| `beta` | `BetaData` | v2.0.0 | current, with a | pipe in the note |
| `ghost` | `GhostData` | v1.0.0 | not in the catalog |
| `beta` | `BetaData` | v2.0.0 | a second row |

Prose after the table, with `gamma` in it.
";
    let rows = table_rows(doc);
    assert_eq!(
        rows.len(),
        4,
        "the rule under the header is not a row: {rows:?}"
    );

    let problems = drift(&rows, &current, &["gamma"]);
    let expect = [
        "`alpha`: the table says v1.1.0, the manifest says v1.2.0",
        "`ghost` has a row but is not in the catalog",
        "`beta` has more than one row",
        "`delta` is in the catalog and has neither a row nor a section of its own",
    ];
    for want in expect {
        assert!(
            problems.iter().any(|p| p == want),
            "missed: {want}\n got: {problems:#?}"
        );
    }
    assert_eq!(
        problems.len(),
        expect.len(),
        "unexpected extras: {problems:#?}"
    );

    // An exemption that a row contradicts is stale.
    let stale = drift(&[("gamma".into(), "v1.0.0".into())], &current, &["gamma"]);
    assert!(
        stale.iter().any(|p| p.contains("as well")),
        "a group with a row must not also be exempt: {stale:?}"
    );

    // And a table that is right is passed.
    let clean_rows: Vec<(String, String)> =
        [("alpha", "v1.2.0"), ("beta", "v2.0.0"), ("delta", "v3.1.0")]
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v.to_owned()))
            .collect();
    assert!(drift(&clean_rows, &current, &["gamma"]).is_empty());
}

/// The same for the section headings.
#[test]
fn the_heading_check_catches_what_it_is_for() {
    let current: BTreeMap<String, String> = [
        ("alpha", "1.2.0"),
        ("beta", "2.0.0"),
        ("gamma", "1.0.0"),
        ("delta", "3.1.0"),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_owned(), v.to_owned()))
    .collect();
    let sections = [
        ("alpha", "AlphaData"),
        ("beta", "BetaData"),
        ("gamma", "GammaData"),
        ("delta", "DeltaData"),
        ("ghost", "GhostData"),
    ];

    let doc = "\
### 4.1 Alpha Product group (`AlphaData`) — v1.1.0

prose

### 4.2 Beta Product group (`BetaData`) — v2.0.0

prose

### 4.3 Delta Product group (`DeltaData`)

prose

### 4.4 Ghost Product group (`GhostData`) — v1.0.0

Gamma has no heading at all, and a heading that merely mentions (`BetaData`) in
prose is not one.
";
    let problems = heading_drift(doc, &sections, &current);
    let expect = [
        "`alpha`: the heading says v1.1.0, the manifest says v1.2.0 — re-read the section \
         below it before changing the number",
        "`gamma` should have exactly one section heading naming `GammaData`, found 0",
        "`delta`: the heading states no version: ### 4.3 Delta Product group (`DeltaData`)",
        "`ghost` has a section but is not in the catalog",
    ];
    for want in expect {
        assert!(
            problems.iter().any(|p| p == want),
            "missed: {want}\n got: {problems:#?}"
        );
    }
    assert_eq!(
        problems.len(),
        expect.len(),
        "unexpected extras: {problems:#?}"
    );

    // Two headings for one type is as wrong as none.
    let twice = "### 1 A (`AlphaData`) — v1.2.0\n### 2 A again (`AlphaData`) — v1.2.0\n";
    let found = heading_drift(twice, &[("alpha", "AlphaData")], &current);
    assert!(
        found.iter().any(|p| p.contains("found 2")),
        "a repeated heading must be reported: {found:?}"
    );

    // And a right heading is passed.
    let right = "### 4.1 Alpha Product group (`AlphaData`) — v1.2.0\n";
    assert!(heading_drift(right, &[("alpha", "AlphaData")], &current).is_empty());
}

#[test]
#[should_panic(expected = "has no table headed")]
fn a_missing_table_fails_loudly() {
    let _ = table_rows("# A document with no table at all\n");
}
