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
//! # What this cannot prove
//!
//! That a recorded status is **current**. The gate reads a file, not the IETF.
//! What it proves is that each citation has a status somebody read and dated.
//! That turns the release-time re-read (`docs/governance/RELEASE.md`) into a
//! walk down one table instead of a search. W3C, GS1, IDTA and ETSI identifiers
//! take too many shapes to match reliably, so the re-read is the only check on
//! their rows.
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
