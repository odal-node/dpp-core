//! A doc comment may name an act freely. It may not attribute a **quantity** to
//! one without saying where the quantity came from.
//!
//! # The gap this closes
//!
//! `dpp-domain`'s schema-prose gate resolves every act number in a JSON
//! `description` to a CELEX identifier and requires it to be known. It iterates
//! `schema_prose()` only. Rust doc comments — `//!` module headers and `///`
//! item docs — carry act citations that no gate had ever read, **including
//! numeric thresholds**, which is the same defect class one surface along.
//!
//! `CLAUDE.md` is explicit that a secondary-sourced claim may never become a
//! constant, threshold or enumerated category. A doc comment is where a number
//! lives immediately before it becomes one.
//!
//! # The rule, and why it is this rule
//!
//! Naming an act is free: `"carried forward from Directive 2006/66/EC"` is
//! provenance, not a claim, and a gate that demanded a source for it would
//! produce noise and then be switched off.
//!
//! **Attributing a quantity is not free.** A doc block that cites an act *and*
//! asserts a percentage, a period in years or months, or a calendar date must
//! carry a `COMPLIANCE-PIN` line — the marker this repository already uses for
//! "this was read against the Official Journal", in either its ✅ or its
//! ⚠️ PENDING form. Both are acceptable, and that is deliberate: the gate's job
//! is to force the question *where did this number come from*, not to pretend a
//! pending answer is a bad one. An honest `PENDING` is a passing answer.
//!
//! # What this cannot prove
//!
//! That a pinned number is **right**. A `COMPLIANCE-PIN` is a claim like any
//! other and this test cannot open a PDF. It proves the question was asked and
//! its answer recorded where the next reader will find it, which turns an audit
//! from research into a lookup — the same limit the schema-prose gate documents
//! about itself.
//!
//! # Why it lives here
//!
//! It must see `dpp-rules`, which `dpp-domain` deliberately does not depend on,
//! so neither crate can host it. `dpp-tests` is the cross-crate tier and already
//! reads `.rs` files structurally for the same kind of reason.

use std::fs;
use std::path::{Path, PathBuf};

use dpp_domain::schemas::citation::act_refs;

/// Crate source roots this gate reads.
const ROOTS: &[&str] = &["../dpp-rules/src", "../dpp-domain/src"];

/// The marker that answers "where did this number come from", in either form.
const PIN: &str = "COMPLIANCE-PIN";

/// Files that attributed a quantity to an act before this gate existed.
///
/// **An inventory that may only shrink, not a suppression list.** Every entry is
/// a real finding: each of these files states a share, a period or a date and
/// attributes it to an EU act, with nothing recording where the figure was read.
/// They are listed rather than fixed because pinning one means reading an
/// Official Journal text, and nineteen of those is not a code change.
///
/// Two properties make this safe to have:
///
/// - **A new file cannot join it by accident.** Anything not listed fails the
///   moment it is written, which is the whole purpose of the gate.
/// - **An entry cannot outlive its finding.** A listed file that has since
///   gained a pin fails [`the_unpinned_inventory_does_not_go_stale`], so the
///   list shrinks as the work is done and can never quietly become a permanent
///   exemption.
///
/// Deliberately paths only, with no per-file reason. A reason here would have to
/// be written without reading the act it concerns, which is the exact failure
/// this gate exists to catch; "cites an act, states a figure, carries no pin" is
/// what is known about every entry and it is already said above.
const UNPINNED_LEGACY: &[&str] = &[
    "dpp-domain/src/catalog/status.rs",
    "dpp-domain/src/instrument/currency/state.rs",
    "dpp-domain/src/operator/basis.rs",
    "dpp-domain/src/operator/responsible.rs",
    "dpp-domain/src/ports/passport_repo/mod.rs",
    "dpp-domain/src/ports/passport_repo/port.rs",
    "dpp-domain/src/ports/seal/mod.rs",
    "dpp-domain/src/product_group/data/battery/state_of_health.rs",
    "dpp-domain/src/product_group/data/detergent.rs",
    "dpp-domain/src/product_group/data/tyre.rs",
    "dpp-domain/src/product_group/data/unsold_goods/financial_year.rs",
    "dpp-domain/src/product_group/data/unsold_goods/mod.rs",
    "dpp-domain/src/product_group/data/unsold_goods/reason.rs",
    "dpp-domain/src/trusted_list/mod.rs",
    "dpp-rules/src/batteries/chemistry.rs",
    "dpp-rules/src/batteries/passport_scope.rs",
    "dpp-rules/src/metals/aluminium.rs",
    "dpp-rules/src/metals/steel.rs",
];

fn manifest_relative(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(rel)
}

/// A path as `UNPINNED_LEGACY` spells it: relative to `crates/`, forward slashes.
fn repo_relative(file: &Path) -> String {
    file.strip_prefix(manifest_relative(".."))
        .unwrap_or(file)
        .display()
        .to_string()
        .replace('\\', "/")
}

/// Every `.rs` file under `dir` that is not itself a test module.
///
/// Test modules are excluded because a test's doc comment describes the test,
/// and a fixture quoting a threshold is not the crate asserting one.
fn source_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            source_files(&path, out);
            continue;
        }
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if name.ends_with(".rs") && name != "tests.rs" && !name.ends_with("_tests.rs") {
            out.push(path);
        }
    }
}

/// One run of consecutive doc-comment lines, with the line it starts on.
struct DocBlock {
    line: usize,
    text: String,
}

/// Group consecutive `///` and `//!` lines into blocks.
///
/// Consecutive matters: the pin and the number have to be in the *same* comment
/// for the pin to be about the number. A pin three items up the file explains
/// nothing about this one.
fn doc_blocks(src: &str) -> Vec<DocBlock> {
    let mut blocks: Vec<DocBlock> = Vec::new();
    let mut current: Option<DocBlock> = None;

    for (index, raw) in src.lines().enumerate() {
        let trimmed = raw.trim_start();
        let content = trimmed
            .strip_prefix("///")
            .or_else(|| trimmed.strip_prefix("//!"));

        match content {
            Some(rest) => match current.as_mut() {
                Some(block) => {
                    block.text.push(' ');
                    block.text.push_str(rest.trim());
                }
                None => {
                    current = Some(DocBlock {
                        line: index + 1,
                        text: rest.trim().to_owned(),
                    });
                }
            },
            None => {
                if let Some(block) = current.take() {
                    blocks.push(block);
                }
            }
        }
    }
    if let Some(block) = current.take() {
        blocks.push(block);
    }
    blocks
}

/// Whether `text` asserts a quantity — a share, a period, or a calendar date.
///
/// Conservative on purpose. It looks for the shapes a regulatory threshold
/// actually takes in this corpus, and not for bare numbers: an article number, a
/// point number and a version string are all digits, and treating them as
/// thresholds would fail the gate on every correct citation in the repository.
fn asserts_a_quantity(text: &str) -> bool {
    let bytes = text.as_bytes();

    // A share: a digit run (with an optional decimal comma or point) followed by
    // optional space and `%`.
    let has_percentage = text.match_indices('%').any(|(at, _)| {
        let before = text[..at].trim_end();
        before
            .chars()
            .last()
            .is_some_and(|c| c.is_ascii_digit() || c == ',' || c == '.')
    });

    // A period: a digit run immediately before "year"/"month"/"day".
    let has_period = ["year", "month", "day"].iter().any(|unit| {
        text.match_indices(unit).any(|(at, _)| {
            text[..at]
                .trim_end()
                .rsplit(|c: char| !c.is_ascii_digit())
                .next()
                .is_some_and(|run| !run.is_empty())
        })
    });

    // A calendar date: `2031-08-18`, or a four-digit year preceded by a month
    // name. A bare year is not enough — "the 2026 working plan" is not a
    // threshold.
    const MONTHS: &[&str] = &[
        "January",
        "February",
        "March",
        "April",
        "May",
        "June",
        "July",
        "August",
        "September",
        "October",
        "November",
        "December",
    ];
    let has_iso_date = (0..bytes.len().saturating_sub(9)).any(|i| {
        let window = &bytes[i..i + 10];
        window[..4].iter().all(u8::is_ascii_digit)
            && window[4] == b'-'
            && window[5..7].iter().all(u8::is_ascii_digit)
            && window[7] == b'-'
            && window[8..].iter().all(u8::is_ascii_digit)
    });
    let has_named_date = MONTHS.iter().any(|month| {
        text.match_indices(month).any(|(at, _)| {
            let after = text[at + month.len()..].trim_start();
            after.chars().take(4).filter(|c| c.is_ascii_digit()).count() == 4
        })
    });

    has_percentage || has_period || has_iso_date || has_named_date
}

/// A doc comment that attributes a quantity to an act says where it came from.
#[test]
fn a_quantity_attributed_to_an_act_names_its_source() {
    let mut offenders: Vec<String> = Vec::new();

    for root in ROOTS {
        let dir = manifest_relative(root);
        let mut files = Vec::new();
        source_files(&dir, &mut files);
        assert!(
            !files.is_empty(),
            "{} yielded no source files — the gate would pass by reading nothing",
            dir.display()
        );

        for file in files {
            let src = fs::read_to_string(&file)
                .unwrap_or_else(|e| panic!("read {}: {e}", file.display()));
            // The pin may sit anywhere in the file, not only in the block that
            // states the number. That matches how the marker is actually used
            // here — a module header introduces an act and the pin sits on the
            // constant it governs, tens of lines below. Requiring them in one
            // comment would fail on correct files, and a gate that fails on
            // correct files gets deleted rather than obeyed.
            if src.contains(PIN) {
                continue;
            }
            let relative = repo_relative(&file);
            if UNPINNED_LEGACY.contains(&relative.as_str()) {
                continue;
            }
            for block in doc_blocks(&src) {
                if act_refs(&block.text).is_empty() || !asserts_a_quantity(&block.text) {
                    continue;
                }
                offenders.push(format!("{relative}:{}", block.line));
            }
        }
    }

    assert!(
        offenders.is_empty(),
        "{} doc comment(s) attribute a number, period or date to an EU act with \
         no {PIN} recording where it was read. Either pin it against the Official \
         Journal text, or mark it {PIN} PENDING and say the number is not \
         sourced yet:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}

/// An entry in the legacy inventory is still a real, unfixed finding.
///
/// Without this the list is a suppression list: a file could be pinned, or
/// deleted, or lose its citation entirely, and the entry would sit there
/// forever, read as a record of work outstanding that is not. The same reasoning
/// makes `CITED_NOT_MODELLED` fail on an act no schema cites any more.
///
/// It fails in both directions, and the first is the one that matters: an entry
/// whose file **now carries a pin** must be removed, so the list can only
/// shrink.
#[test]
fn the_unpinned_inventory_does_not_go_stale() {
    let mut fixed: Vec<&str> = Vec::new();
    let mut missing: Vec<&str> = Vec::new();

    for entry in UNPINNED_LEGACY {
        let path = manifest_relative("..").join(entry);
        let Ok(src) = fs::read_to_string(&path) else {
            missing.push(entry);
            continue;
        };
        if src.contains(PIN) {
            fixed.push(entry);
            continue;
        }
        let still_offends = doc_blocks(&src)
            .iter()
            .any(|b| !act_refs(&b.text).is_empty() && asserts_a_quantity(&b.text));
        if !still_offends {
            fixed.push(entry);
        }
    }

    assert!(
        missing.is_empty(),
        "UNPINNED_LEGACY names {} file(s) that no longer exist — remove them: {missing:?}",
        missing.len()
    );
    assert!(
        fixed.is_empty(),
        "{} file(s) in UNPINNED_LEGACY no longer attribute an unsourced quantity \
         to an act. Remove them from the list — leaving them turns an inventory \
         of outstanding work into a permanent exemption: {fixed:?}",
        fixed.len()
    );
}

/// The gate is only worth having if it fails on the shape it exists to catch.
///
/// Asserted on the detector directly, so the evidence lives beside the rule
/// rather than in a commit message somebody has to find.
#[test]
fn the_detector_catches_what_it_is_for() {
    // The shape this gate was written for: an act, a period, no pin.
    let offending = "Minimum spare-parts availability is 10 years under \
                     Regulation (EU) 2019/2022.";
    assert!(!act_refs(offending).is_empty());
    assert!(asserts_a_quantity(offending));

    // Shares and dates, in the forms this corpus uses.
    for text in [
        "16 % cobalt from Regulation (EU) 2023/1542.",
        "0,0005 % mercury under Regulation (EU) 2023/1542.",
        "Applies from 18 August 2031 per Regulation (EU) 2023/1542.",
        "Placed on the market from 2031-08-18 under Regulation (EU) 2023/1542.",
    ] {
        assert!(asserts_a_quantity(text), "missed a quantity in: {text}");
    }

    // Naming an act is free. None of these may fire, or the gate produces noise
    // and gets switched off rather than fixed.
    for text in [
        "Carried forward from Directive 2006/66/EC, now repealed.",
        "Annex XIII point 1(q) of Regulation (EU) 2023/1542 enumerates them.",
        "Art. 77(7) of Regulation (EU) 2023/1542 names four operations.",
        "Ranked 1/2 in the 2026 working plan.",
    ] {
        assert!(
            !asserts_a_quantity(text),
            "false positive — this asserts no quantity: {text}"
        );
    }
}
