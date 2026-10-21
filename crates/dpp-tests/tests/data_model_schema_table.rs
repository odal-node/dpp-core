//! Drift tripwire: the "Current schema" column of the table in
//! `docs/architecture/DATA-MODEL.md` §4.3 must name the version each product
//! group's manifest declares current.
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
//! out of the document unnoticed.
//!
//! The headings of those two sections also state a version. This test does not
//! read them: a heading claims that the section below it describes that version's
//! shape, which is a statement about the prose and not only about a number.
//!
//! Lives here and not in `dpp-domain` for the reason `schema_changes.rs` does:
//! it reads a path outside any crate, and `dpp-domain` is published.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use dpp_domain::catalog::ProductGroupCatalog;

/// Product groups documented in a section of their own above the table.
const GROUPS_WITH_THEIR_OWN_SECTION: &[&str] = &["battery", "textile"];

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

#[test]
fn data_model_schema_column_matches_the_catalog() {
    let catalog = ProductGroupCatalog::new();
    let current: BTreeMap<String, String> = catalog
        .all()
        .iter()
        .map(|d| (d.key.clone(), d.current_schema_version.clone()))
        .collect();
    let rows = table_rows(&data_model());

    // A parser that quietly stops matching would let this pass while checking
    // nothing, which is how a drift gate fails.
    assert!(
        rows.len() >= 5,
        "parsed {} rows from the DATA-MODEL.md table — the parser or the table has changed shape: {rows:?}",
        rows.len()
    );

    let problems = drift(&rows, &current, GROUPS_WITH_THEIR_OWN_SECTION);
    assert!(
        problems.is_empty(),
        "\nthe DATA-MODEL.md schema table has drifted from the product group manifests — \
         update the \"Current schema\" cells in §4.3.\n  {}\n",
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

#[test]
#[should_panic(expected = "has no table headed")]
fn a_missing_table_fails_loudly() {
    let _ = table_rows("# A document with no table at all\n");
}
