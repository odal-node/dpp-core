//! Every IETF specification this repository cites has a row in the standards
//! register, and the row says what its status was when it was last read.
//!
//! # The gap this closes
//!
//! Acts have a gate: `doc_comment_citations.rs` makes a doc comment that
//! attributes a quantity to an act say where the quantity was read. Technical
//! specifications had nothing. No file recorded which revision was cited or what
//! its status was, and the one status that was written down lived in two tables
//! that drifted apart: one called W3C VC Data Model 2.0 "Published", the other
//! still called it a Candidate Recommendation, a year after it became a
//! Recommendation.
//!
//! An RFC can be obsoleted, and an Internet-Draft can gain a revision or become
//! an RFC. A citation written against either goes stale silently. It still
//! compiles, and it still reads as true.
//!
//! # The rules
//!
//! These apply to every `RFC NNNN` and `draft-ietf-…` cited anywhere this gate
//! reads:
//!
//! 1. **A citation has a row** in `docs/architecture/STANDARDS.md`. A new
//!    citation therefore forces its status to be read when it is written,
//!    rather than being discovered later.
//! 2. **An obsoleted specification is not cited** unless its row gives a
//!    `kept:` reason. Otherwise the fix is to cite the successor.
//! 3. **An Internet-Draft is cited with its revision** (`-NN`), and its row
//!    claims no conformance. A draft name without a revision points at a moving
//!    text, and a draft is not reference material anyone can conform to.
//! 4. **A row is cited somewhere.** A row nothing cites records the status of a
//!    text this repository no longer relies on, and every release would re-read
//!    it for nothing.
//!
//! Two more rules apply to every row of the register, in both tables, and not
//! only to the IETF rows:
//!
//! 5. **A row carries a status and the date it was read.** An empty `Status as
//!    read`, or a `Read` that is not a `YYYY-MM-DD` date, fails. Without this the
//!    promise below, that each citation has a status somebody read and dated,
//!    would hold only until a cell was cleared.
//! 6. **A conformance claim names its evidence.** A `Conformance claimed` cell
//!    that starts with `Yes` must name at least one repository path, every path
//!    it names must exist, and the row's `Evidence` cell must offer more than
//!    unit tests. Unit tests check this code against its own reading of a
//!    specification, so they can never carry a claim alone. What a claim must
//!    say is in the register.
//!
//! # What this cannot prove
//!
//! That a recorded status is **current**. The gate reads a file, not the IETF.
//! What it proves is that each IETF citation has a row, and that every row has a
//! status somebody read and dated. Likewise it holds a claim to the form that
//! makes it checkable, a named and existing path, and not to its truth: whether
//! that path is evidence that is not circular is a reviewer's judgement. That turns the release-time re-read
//! (`docs/governance/RELEASE.md`) into a walk down one table instead of a
//! search. W3C, GS1, IDTA and ETSI identifiers take too many shapes to match
//! reliably, so nothing checks that their rows are still cited: the re-read is
//! the only check on those.
//!
//! # What it reads
//!
//! Every `.rs`, `.md`, `.toml`, `.json`, `.yml` and `.yaml` file in the
//! repository, plus `.gitattributes` and `justfile`. It skips build output and
//! hidden directories other than `.github`. Three files are skipped:
//!
//! - the register itself, which is what citations are checked against;
//! - `CHANGELOG.md`, a record of what was said at each release, since
//!   rewriting a released entry changes no claim made today;
//! - this file, whose fixtures cite on purpose.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The register, relative to the workspace root.
const REGISTER: &str = "docs/architecture/STANDARDS.md";

/// Files this gate does not read, each for the reason in the module docs.
const SKIPPED: &[&str] = &[
    REGISTER,
    "CHANGELOG.md",
    "crates/dpp-tests/tests/standard_citations.rs",
];

const EXTENSIONS: &[&str] = &["rs", "md", "toml", "json", "yml", "yaml"];

const NAMED_FILES: &[&str] = &[".gitattributes", "justfile"];

/// A register row whose specification is an IETF document.
struct Row {
    /// `RFC 9901`, or a draft's full name with its revision.
    spec: String,
    status: String,
    conformance: String,
    /// The whole line, for its `kept:` reason.
    text: String,
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn starts_a_word(line: &str, at: usize) -> bool {
    !line[..at].chars().next_back().is_some_and(is_word_char)
}

/// Every IETF specification `line` cites, spelled as the register spells it.
///
/// Case-sensitive on purpose: `rfc9901_vector_tests` is a file name, not a
/// citation, and JSON Schema's `draft-07` is not an Internet-Draft name.
fn citations(line: &str) -> Vec<String> {
    let mut found = Vec::new();

    for (at, _) in line.match_indices("RFC") {
        if !starts_a_word(line, at) {
            continue;
        }
        let rest = &line[at + 3..];
        let rest = rest.strip_prefix(' ').unwrap_or(rest);
        let digits = rest.len() - rest.trim_start_matches(|c: char| c.is_ascii_digit()).len();
        let ends_cleanly = !rest[digits..].chars().next().is_some_and(is_word_char);
        if (3..=4).contains(&digits) && ends_cleanly {
            found.push(format!("RFC {}", &rest[..digits]));
        }
    }

    for (at, _) in line.match_indices("draft-ietf-") {
        if !starts_a_word(line, at) {
            continue;
        }
        let name: String = line[at..]
            .chars()
            .take_while(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || *c == '-')
            .collect();
        found.push(name.trim_end_matches('-').to_owned());
    }

    found
}

/// Whether a draft name ends in its two-digit revision.
fn has_revision(draft: &str) -> bool {
    draft
        .rsplit_once('-')
        .is_some_and(|(_, rev)| rev.len() == 2 && rev.bytes().all(|b| b.is_ascii_digit()))
}

/// A Markdown table line split into trimmed cells, or `None` if it is not one.
fn cells(line: &str) -> Option<Vec<String>> {
    let inner = line.trim().strip_prefix('|')?.strip_suffix('|')?;
    Some(inner.split('|').map(|c| c.trim().to_owned()).collect())
}

/// The register's IETF rows, read from the table whose header opens `| Spec |`.
fn register() -> Vec<Row> {
    let path = workspace_root().join(REGISTER);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    let mut header: Option<Vec<String>> = None;
    let mut rows = Vec::new();
    for line in text.lines() {
        let Some(row) = cells(line) else {
            header = None;
            continue;
        };
        if row.first().is_some_and(|c| c == "Spec") {
            header = Some(row);
            continue;
        }
        let Some(columns) = header.as_ref() else {
            continue;
        };
        let spec = row[0].trim_matches('`').to_owned();
        if !(spec.starts_with("RFC ") || spec.starts_with("draft-")) {
            continue;
        }
        let column = |name: &str| {
            let index = columns
                .iter()
                .position(|c| c == name)
                .unwrap_or_else(|| panic!("{REGISTER}: the IETF table has no `{name}` column"));
            row.get(index).cloned().unwrap_or_default()
        };
        rows.push(Row {
            status: column("Status as read"),
            conformance: column("Conformance claimed"),
            text: line.to_owned(),
            spec,
        });
    }

    assert!(
        !rows.is_empty(),
        "{REGISTER} yielded no IETF rows — the gate would pass by reading nothing"
    );
    rows
}

/// One data row of one register table: its first cell, and every cell by the
/// name of its column.
struct RegisterRow {
    name: String,
    cells: BTreeMap<String, String>,
}

impl RegisterRow {
    /// A cell's text, or `None` if the row's table has no such column.
    fn cell(&self, column: &str) -> Option<&str> {
        self.cells.get(column).map(String::as_str)
    }
}

/// Every data row of every table in the register.
///
/// Reads both tables, and fails if either goes unread: a gate that quietly
/// stopped seeing one of them would keep passing.
fn register_rows() -> Vec<RegisterRow> {
    let path = workspace_root().join(REGISTER);
    let text =
        fs::read_to_string(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));

    let mut header: Option<Vec<String>> = None;
    let mut tables = Vec::new();
    let mut rows = Vec::new();
    for line in text.lines() {
        let Some(row) = cells(line) else {
            header = None;
            continue;
        };
        let Some(columns) = header.as_ref() else {
            tables.push(row[0].clone());
            header = Some(row);
            continue;
        };
        if row
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ':'))
        {
            continue;
        }
        rows.push(RegisterRow {
            name: row[0].clone(),
            cells: columns.iter().cloned().zip(row).collect(),
        });
    }

    for table in ["Spec", "Specification"] {
        assert!(
            tables.iter().any(|t| t == table),
            "{REGISTER} has no table whose first column is `{table}` — the gate would pass by not reading it"
        );
    }
    rows
}

/// The repository paths a conformance cell names: every backticked token that
/// contains a `/` and is not a URL.
fn repository_paths(cell: &str) -> Vec<&str> {
    cell.split('`')
        .skip(1)
        .step_by(2)
        .filter(|token| token.contains('/') && !token.contains("://") && !token.contains(' '))
        .collect()
}

/// Whether an Evidence cell offers nothing but unit tests.
fn only_unit_tests(evidence: &str) -> bool {
    evidence
        .to_ascii_lowercase()
        .replace("unit tests", "")
        .chars()
        .all(|c| !c.is_alphanumeric())
}

/// What is wrong with a row's conformance claim. A cell that does not start with
/// `Yes` makes no claim and has nothing to check.
///
/// This holds a claim to the form that makes it checkable, not to the truth of
/// it. That the named evidence is not circular stays a reviewer's call.
fn claim_faults(conformance: &str, evidence: &str, root: &Path) -> Vec<String> {
    if !conformance.starts_with("Yes") {
        return Vec::new();
    }
    let mut faults = Vec::new();
    let paths = repository_paths(conformance);
    if paths.is_empty() {
        faults.push("names no repository path as evidence".to_owned());
    }
    for path in paths {
        if !root.join(path).exists() {
            faults.push(format!("names `{path}`, which does not exist"));
        }
    }
    if only_unit_tests(evidence) {
        faults.push("its only evidence is unit tests".to_owned());
    }
    faults
}

/// Every file this gate reads.
fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let entries =
        fs::read_dir(dir).unwrap_or_else(|e| panic!("cannot scan {}: {e}", dir.display()));
    for entry in entries {
        let path = entry
            .unwrap_or_else(|e| panic!("cannot scan {}: {e}", dir.display()))
            .path();
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if path.is_dir() {
            let hidden = name.starts_with('.') && name != ".github";
            if !hidden && name != "target" {
                walk(&path, out);
            }
        } else if NAMED_FILES.contains(&name)
            || path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| EXTENSIONS.contains(&e))
        {
            out.push(path);
        }
    }
}

/// Every cited specification, with each `path:line` that cites it.
fn cited() -> BTreeMap<String, Vec<String>> {
    let root = workspace_root();
    let mut files = Vec::new();
    walk(&root, &mut files);
    assert!(
        files.len() > 100,
        "found {} files — has the layout moved?",
        files.len()
    );

    let mut cited: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for file in files {
        let relative = file
            .strip_prefix(&root)
            .unwrap_or(&file)
            .display()
            .to_string()
            .replace('\\', "/");
        if SKIPPED.contains(&relative.as_str()) {
            continue;
        }
        let text = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()));
        for (index, line) in text.lines().enumerate() {
            for spec in citations(line) {
                cited
                    .entry(spec)
                    .or_default()
                    .push(format!("{relative}:{}", index + 1));
            }
        }
    }
    cited
}

fn listing(cited: &BTreeMap<String, Vec<String>>, specs: &[&String]) -> String {
    specs
        .iter()
        .map(|spec| format!("  {spec}: {}", cited[spec.as_str()].join(", ")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Rules 1 and 3: every citation has a row, and a draft is cited with its
/// revision.
#[test]
fn every_cited_ietf_specification_has_a_register_row() {
    let rows = register();
    let cited = cited();

    let unversioned: Vec<&String> = cited
        .keys()
        .filter(|spec| spec.starts_with("draft-") && !has_revision(spec))
        .collect();
    assert!(
        unversioned.is_empty(),
        "an Internet-Draft is cited without its revision number, which names a \
         moving text. Cite it as `name-NN`:\n{}",
        listing(&cited, &unversioned)
    );

    let unregistered: Vec<&String> = cited
        .keys()
        .filter(|spec| !rows.iter().any(|row| &row.spec == *spec))
        .collect();
    assert!(
        unregistered.is_empty(),
        "cited with no row in {REGISTER}. Read each one's current status at \
         https://www.rfc-editor.org/info/rfcNNNN (or the datatracker, for a \
         draft) and add a dated row:\n{}",
        listing(&cited, &unregistered)
    );
}

/// Rule 2: an obsoleted specification is cited only with a stated reason.
#[test]
fn an_obsoleted_specification_is_cited_only_with_a_reason() {
    let cited = cited();
    let offenders: Vec<&String> = register()
        .iter()
        .filter(|row| row.status.contains("Obsoleted by") && !row.text.contains("kept:"))
        .filter_map(|row| cited.get_key_value(&row.spec).map(|(spec, _)| spec))
        .collect();
    assert!(
        offenders.is_empty(),
        "cited although {REGISTER} records it as obsoleted. Cite the successor, \
         or give the row a `kept:` reason saying why this one must stay:\n{}",
        listing(&cited, &offenders)
    );
}

/// Rule 3: an Internet-Draft's row claims no conformance.
#[test]
fn a_draft_row_claims_no_conformance() {
    let claimed: Vec<String> = register()
        .into_iter()
        .filter(|row| row.spec.starts_with("draft-") && row.conformance != "No")
        .map(|row| format!("{} ({})", row.spec, row.conformance))
        .collect();
    assert!(
        claimed.is_empty(),
        "an Internet-Draft is not reference material to conform to, so its row \
         must say `No` under Conformance claimed: {claimed:?}"
    );
}

/// Rule 4: every row is cited somewhere, so the register cannot go stale.
#[test]
fn every_register_row_is_still_cited() {
    let cited = cited();
    let stale: Vec<String> = register()
        .into_iter()
        .filter(|row| !cited.contains_key(&row.spec))
        .map(|row| row.spec)
        .collect();
    assert!(
        stale.is_empty(),
        "{REGISTER} has rows nothing cites any more. Remove them, or they will be \
         re-read at every release for a text this repository no longer relies on: \
         {stale:?}"
    );
}

/// Rule 5: every row carries a status and the date it was read.
#[test]
fn every_register_row_carries_a_status_and_a_read_date() {
    let faulty: Vec<String> = register_rows()
        .into_iter()
        .filter_map(|row| {
            let (Some(status), Some(read)) = (row.cell("Status as read"), row.cell("Read")) else {
                return None;
            };
            let mut faults = Vec::new();
            if status.is_empty() {
                faults.push("no `Status as read`");
            }
            let a_date =
                read.len() == 10 && chrono::NaiveDate::parse_from_str(read, "%Y-%m-%d").is_ok();
            if !a_date {
                faults.push("`Read` is not a YYYY-MM-DD date");
            }
            (!faults.is_empty()).then(|| format!("  {}: {}", row.name, faults.join(", ")))
        })
        .collect();
    assert!(
        faulty.is_empty(),
        "{REGISTER} has rows without a dated status. Read the specification's \
         current status at its source and record it with the date:\n{}",
        faulty.join("\n")
    );
}

/// Rule 6: a conformance claim names evidence that exists, and is not backed by
/// unit tests alone.
#[test]
fn a_conformance_claim_names_its_evidence() {
    let root = workspace_root();
    let faulty: Vec<String> = register_rows()
        .into_iter()
        .filter_map(|row| {
            let faults = claim_faults(
                row.cell("Conformance claimed")?,
                row.cell("Evidence").unwrap_or_default(),
                &root,
            );
            (!faults.is_empty()).then(|| format!("  {}: {}", row.name, faults.join("; ")))
        })
        .collect();
    assert!(
        faulty.is_empty(),
        "{REGISTER} has a `Yes` that is not held to evidence. A claim names, as \
         repository paths in its own cell, the specification's test vectors, an \
         external suite or an independent implementation it was run against, and \
         unit tests alone cannot carry it:\n{}",
        faulty.join("\n")
    );
}

/// The claim check is only worth having if it fails on each way a claim can lack
/// evidence, and passes on one that has it.
#[test]
fn the_claim_check_catches_what_it_is_for() {
    let root = workspace_root();
    let real = "crates/dpp-tests/tests/standard_citations.rs";
    let backed = format!("Yes. Self-declared; run against `{real}`.");

    assert!(claim_faults(&backed, "The RFC's own test vectors", &root).is_empty());
    assert!(
        claim_faults("No", "Unit tests", &root).is_empty(),
        "a `No` makes no claim, so there is nothing to hold to evidence"
    );

    assert_eq!(
        claim_faults("Yes", "The RFC's own test vectors", &root),
        ["names no repository path as evidence"]
    );
    assert_eq!(
        claim_faults(
            "Yes. Run against `crates/nowhere/missing.rs`.",
            "Vectors",
            &root
        ),
        ["names `crates/nowhere/missing.rs`, which does not exist"]
    );
    for evidence in ["Unit tests", "unit tests.", "Unit tests; unit tests", ""] {
        assert_eq!(
            claim_faults(&backed, evidence, &root),
            ["its only evidence is unit tests"],
            "evidence: {evidence:?}"
        );
    }

    // A URL and a bare file name are not repository paths.
    assert_eq!(
        repository_paths("Yes, per `https://example.test/a/b` and `jades-oracle.yml`"),
        Vec::<&str>::new()
    );
}

/// The detector is only worth having if it finds the shapes citations take, and
/// nothing else.
#[test]
fn the_detector_catches_what_it_is_for() {
    for (line, expected) in [
        ("per RFC 7807 §3", vec!["RFC 7807"]),
        ("(RFC 8785) canonical", vec!["RFC 8785"]),
        ("`RFC 9901`'s clause 4", vec!["RFC 9901"]),
        ("RFC1918 ranges", vec!["RFC 1918"]),
        ("RFC 3339 / RFC 9557", vec!["RFC 3339", "RFC 9557"]),
        (
            "profile draft-ietf-oauth-sd-jwt-vc-19, read",
            vec!["draft-ietf-oauth-sd-jwt-vc-19"],
        ),
        (
            "see https://datatracker.ietf.org/doc/draft-ietf-oauth-sd-jwt-vc/",
            vec!["draft-ietf-oauth-sd-jwt-vc"],
        ),
    ] {
        assert_eq!(citations(line), expected, "in: {line}");
    }

    assert!(has_revision("draft-ietf-oauth-sd-jwt-vc-19"));
    assert!(!has_revision("draft-ietf-oauth-sd-jwt-vc"));

    // None of these is a citation, and treating any as one would fail the gate
    // on correct files until someone switched it off.
    for line in [
        "mod rfc9901_vector_tests;",
        "\"$schema\": \"http://json-schema.org/draft-07/schema#\"",
        "several RFCs apply",
        "RFC 12345 is not a number the series has reached",
        "a draft-19 profile",
        "XRFC 7515",
    ] {
        assert!(citations(line).is_empty(), "false positive in: {line}");
    }
}
