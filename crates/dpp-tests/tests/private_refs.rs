//! Fail the build when a file in this public repository leaks private material.
//!
//! This repository is public and its crates are published. Anything committed
//! here is readable the moment it is pushed, and a released version can never be
//! unpublished — so a leak is a defect that cannot be walked back, only
//! superseded. `CLAUDE.md` section 6 states the rule; this is the enforcement.
//!
//! It exists because the rule was stated and then broken: a vocabulary record
//! shipped carrying a filesystem path into a private sibling repository,
//! compiled into the published artefact by `include_str!`. Review missed it
//! because the string sat in a JSON data field rather than in code. Every
//! recorded instance of this class has been in **prose** — module docs, schema
//! `description` fields, README paragraphs — so a check that reads only code
//! comments is not a check. This one reads every text file in the tree.
//!
//! # What is severe, and what is merely premature
//!
//! These are not equally bad, and treating them as one rule got the priorities
//! backwards:
//!
//! **Severe — never, anywhere.** A pointer into a private document; a reference
//! to an internal decision record. The first hands a reader a specific thing
//! they are not meant to have; the second discloses that a decision was made,
//! how such decisions are numbered, and that the reasoning lives somewhere they
//! cannot reach.
//!
//! **Severe, and checked from a list this file does not contain.** The name of
//! any client, operator, manufacturer or collaborating party — see below.
//!
//! **Premature rather than dangerous.** The bare name of a sibling repository.
//! These are operating terms that become public eventually; naming one in an
//! internal comment costs nothing. Naming one in a **README or a crate
//! description** is different, because that is the customer-facing surface and
//! crates.io renders it. So the bare names are checked *only* there.
//!
//! # Party names are checked, and the list is not in this file
//!
//! Party names are checked — by [`no_public_file_names_a_private_party`] — but
//! from a list supplied through the environment, never from one written here. A
//! denylist of things that must stay secret cannot live in the repository it is
//! protecting: writing the list *is* the disclosure, and a worse one than the
//! leak it prevents, because it would be authoritative and enumerated rather
//! than incidental.
//!
//! Hashing them into this file is not a fix either. Party names are short and
//! guessable, so a digest is obfuscation rather than secrecy, and calling it
//! secrecy is how a control stops being examined.
//!
//! **The failure message names a file and a line and never the matched text.**
//! This runs in a public repository's CI, and its logs are public, so a check
//! that printed what it found would publish the very thing it exists to keep
//! unpublished — on the exact commit that tried to leak it.
//!
//! # What it deliberately does not look for
//!
//! General planning vocabulary, priority codes, phase letters. Real parts of the
//! rule, and too ambiguous to grep without drowning the signal in false
//! positives — `P0` is legitimate in a dozen contexts. A tripwire people switch
//! off is worse than no tripwire.

use std::path::{Path, PathBuf};

/// Sibling repositories that are not public *yet*.
///
/// Listing them here is deliberate and safe: they are operating terms the
/// project expects to disclose in time, and the rule about them is one of
/// placement rather than secrecy. What must not happen is a **path into** one
/// (that points at a document) or a mention on the customer-facing surface.
const SIBLING_REPOS: &[&str] = &[
    "dpp-docs",
    "dpp-control-plane",
    "dpp-infra",
    "dpp-legacy",
    "dpp-web",
];

/// Extensions worth reading. Prose counts — that is where every recorded
/// instance of this failure class has been.
const TEXT_SUFFIXES: &[&str] = &[
    "rs", "md", "json", "toml", "yml", "yaml", "sh", "py", "mjs", "txt",
];

const SKIP_DIRS: &[&str] = &[".git", "target", "node_modules", ".claude"];

/// Files allowed to carry these patterns, because stating a rule requires
/// naming what it forbids.
///
/// An explicit list rather than an inline opt-out marker: a marker is one edit
/// away from being sprinkled onto a real leak.
const ALLOWLIST: &[&str] = &["crates/dpp-tests/tests/private_refs.rs", "CLAUDE.md"];

/// How bad a finding is, which decides where it is looked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Severity {
    /// Never acceptable, in any file.
    Severe,
    /// Acceptable internally; not on the customer-facing surface.
    CustomerFacingOnly,
}

/// A finding: what matched, why it matters, and how bad it is.
struct Finding {
    rule: &'static str,
    hit: String,
    why: &'static str,
    severity: Severity,
}

/// Whether `needle` occurs in `haystack` delimited the way a regex `\b` would.
///
/// Hand-rolled because the workspace has no regex dependency and these patterns
/// do not justify adding one. A word character is alphanumeric or `_`, matching
/// the regex definition — note `-` is *not* one, which is what makes
/// `dpp-docs` match inside `dpp-docs/reference`.
fn find_word(haystack: &str, needle: &str) -> Option<usize> {
    let is_word = |c: char| c.is_alphanumeric() || c == '_';
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(needle) {
        let start = from + rel;
        let end = start + needle.len();
        let before_ok = haystack[..start]
            .chars()
            .next_back()
            .is_none_or(|c| !is_word(c));
        let after_ok = haystack[end..].chars().next().is_none_or(|c| !is_word(c));
        if before_ok && after_ok {
            return Some(start);
        }
        from = start + 1;
    }
    None
}

/// `ADR-12`, `ADR 007` — a reference to an internal decision record.
fn find_decision_record(line: &str) -> Option<String> {
    let mut from = 0;
    while let Some(rel) = line[from..].find("ADR") {
        let start = from + rel;
        let rest = &line[start + 3..];
        let mut chars = rest.chars();
        if let Some(sep) = chars.next()
            && (sep == '-' || sep.is_whitespace())
            && chars.next().is_some_and(|d| d.is_ascii_digit())
        {
            let tail: String = rest.chars().take(5).collect();
            return Some(format!("ADR{tail}").trim_end().to_owned());
        }
        from = start + 1;
    }
    None
}

/// A mention of a sibling repo immediately followed by `/` — a document pointer.
///
/// This is the shape that actually shipped. The bare name is a placement
/// question; the path is somebody being handed a specific file they cannot open
/// and should not know about.
fn find_private_path(line: &str) -> Option<String> {
    for repo in SIBLING_REPOS {
        if let Some(at) = find_word(line, repo) {
            let after = &line[at + repo.len()..];
            if after.starts_with('/') {
                let tail: String = after.chars().take_while(|c| !c.is_whitespace()).collect();
                let tail: String = tail.chars().take(40).collect();
                return Some(format!("{repo}{tail}"));
            }
        }
    }
    None
}

/// Every rule, applied to one line.
fn scan_line(line: &str) -> Option<Finding> {
    if let Some(hit) = find_private_path(line) {
        return Some(Finding {
            rule: "private-document",
            hit,
            why: "points into a document a public reader cannot open; state the substance inline",
            severity: Severity::Severe,
        });
    }
    if let Some(hit) = find_decision_record(line) {
        return Some(Finding {
            rule: "internal-decision",
            hit,
            why: "cites an internal decision record; cite the OJ text or the standard instead",
            severity: Severity::Severe,
        });
    }
    for repo in SIBLING_REPOS {
        if find_word(line, repo).is_some() {
            return Some(Finding {
                rule: "sibling-repo",
                hit: (*repo).to_owned(),
                why: "names a not-yet-public sibling on the customer-facing surface",
                severity: Severity::CustomerFacingOnly,
            });
        }
    }
    None
}

/// The surface a customer reads: rendered on crates.io and on the repo front
/// page. A `CustomerFacingOnly` finding counts only here.
fn is_customer_facing(rel: &str) -> bool {
    rel == "README.md" || rel.ends_with("/README.md") || rel.ends_with("Cargo.toml")
}

/// Every text file worth reading, as (absolute, repo-relative) pairs.
///
/// Skip directories are matched against the **relative** path. Matching the
/// absolute one silently scans nothing when the checkout itself sits under a
/// skipped name — which is what a git worktree under `.claude/` does, and a
/// scanner reporting success having read nothing is worse than none.
fn text_files(root: &Path) -> Vec<(PathBuf, String)> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];

    while let Some(dir) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(rel) = path.strip_prefix(root) else {
                continue;
            };
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if rel_str.split('/').any(|part| SKIP_DIRS.contains(&part)) {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path
                .extension()
                .and_then(|e| e.to_str())
                .is_some_and(|e| TEXT_SUFFIXES.contains(&e))
            {
                out.push((path, rel_str));
            }
        }
    }

    out.sort_by(|a, b| a.1.cmp(&b.1));
    out
}

#[test]
fn no_file_in_this_public_repo_leaks_private_material() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root is two levels above this crate");

    let mut findings = Vec::new();
    let mut scanned = 0usize;

    for (path, rel) in text_files(&root) {
        if ALLOWLIST.contains(&rel.as_str()) {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue; // not UTF-8: no prose to leak
        };
        scanned += 1;
        for (lineno, line) in text.lines().enumerate() {
            let Some(f) = scan_line(line) else { continue };
            if f.severity == Severity::CustomerFacingOnly && !is_customer_facing(&rel) {
                continue;
            }
            let excerpt: String = line.trim().chars().take(160).collect();
            findings.push(format!(
                "{rel}:{}: [{}] {:?} — {}\n      {excerpt}",
                lineno + 1,
                f.rule,
                f.hit,
                f.why
            ));
        }
    }

    assert!(
        scanned > 0,
        "scanned no files at all — the walk is broken, and a scanner that \
         reports success having read nothing is worse than no scanner"
    );

    assert!(
        findings.is_empty(),
        "Private material found in a public repository ({} across {scanned} files):\n\n  {}\n\n\
         See CLAUDE.md section 6. Write the substance, drop the pointer: the \
         regulatory and technical reasoning is nearly always public-safe; the \
         internal record that decided how to read it is not.",
        findings.len(),
        findings.join("\n\n  ")
    );
}

/// The matcher catches the shapes that actually shipped, and only those.
///
/// Without this, a passing scan means either "the tree is clean" or "the matcher
/// is broken", and those look identical from outside.
#[test]
fn the_matcher_separates_severe_from_premature() {
    // The shape that reached a published crate: a path inside JSON prose. The
    // segments here are invented — reproducing the real ones would put the
    // directory layout of a private repository into a public file, which is the
    // thing this rule exists to stop.
    let hit = scan_line(r#"  "source": "dpp-docs/aaa/bbb.pdf","#)
        .expect("a private document path must be caught");
    assert_eq!(hit.rule, "private-document");
    assert_eq!(hit.severity, Severity::Severe);

    // Internal decisions, both separators, anywhere.
    for line in ["as decided in ADR-010 section 4", "per ADR 007"] {
        let hit = scan_line(line).expect("a decision-record reference must be caught");
        assert_eq!(hit.rule, "internal-decision");
        assert_eq!(hit.severity, Severity::Severe);
    }

    // A bare sibling name is placement, not secrecy — flagged, but only on the
    // customer-facing surface.
    let hit = scan_line("the terraform lives in dpp-infra").expect("a bare mention is a finding");
    assert_eq!(hit.rule, "sibling-repo");
    assert_eq!(hit.severity, Severity::CustomerFacingOnly);
    assert!(is_customer_facing("README.md"));
    assert!(is_customer_facing("crates/dpp-domain/README.md"));
    assert!(is_customer_facing("crates/dpp-domain/Cargo.toml"));
    assert!(!is_customer_facing("crates/dpp-domain/src/lib.rs"));

    // Public siblings are nameable anywhere; this one is named throughout.
    assert!(scan_line("the adapter lives in dpp-engine").is_none());
    assert!(scan_line("dpp-core is Apache-2.0").is_none());
    // `ADR` without a number is not a reference to one.
    assert!(scan_line("the ADR process").is_none());
    // A longer word merely containing a sibling name is not a hit.
    assert!(scan_line("dpp-webhooks is a workspace concept").is_none());
}

/// The environment variable carrying the party-name list, newline-separated.
///
/// In CI it comes from a repository secret. Locally it is whatever the
/// developer exports, and most will export nothing — which is handled by
/// skipping loudly rather than silently.
const PARTY_NAMES_VAR: &str = "ODAL_PARTY_NAMES";

/// Set by the one CI job that owns this check, to say the list must be present.
///
/// Not a `CI` heuristic. This test lives in `dpp-tests`, so it also runs under
/// the ordinary workspace test job — which has no business holding the secret,
/// and would have failed there for the right reason in the wrong place. The flag
/// makes exactly one job responsible, and lets anyone opt into strictness
/// locally.
const PARTY_NAMES_REQUIRED_VAR: &str = "ODAL_PARTY_NAMES_REQUIRED";

/// No file in this public repository names a client, operator, manufacturer or
/// collaborating party.
///
/// # Why the list is injected
///
/// See the module documentation: the names cannot be written down here. They
/// arrive through [`PARTY_NAMES_VAR`], one per line, `#` comments ignored.
///
/// # Why an absent list fails CI
///
/// A check that quietly does nothing is worse than no check, because it reports
/// success. Locally an absent list is ordinary and skips with a message saying
/// what was not checked. In CI it is a misconfiguration — the secret is missing
/// or was renamed — and the honest response is red, not green.
#[test]
fn no_public_file_names_a_private_party() {
    // An unset variable and one carrying an undefined secret are the same
    // situation — GitHub substitutes the empty string for a secret that does not
    // exist — so both take the one path rather than tripping different rules.
    let raw = std::env::var(PARTY_NAMES_VAR).unwrap_or_default();
    let names: Vec<String> = raw
        .lines()
        .map(|l| l.split('#').next().unwrap_or("").trim().to_lowercase())
        .filter(|l| !l.is_empty())
        .collect();

    if names.is_empty() {
        assert!(
            std::env::var_os(PARTY_NAMES_REQUIRED_VAR).is_none(),
            "{PARTY_NAMES_REQUIRED_VAR} is set but {PARTY_NAMES_VAR} holds no names. \
             The party-name scan cannot run, and passing without it would report a \
             clean tree that was never checked — most likely the repository secret \
             this job injects does not exist or was renamed."
        );
        eprintln!(
            "note: {PARTY_NAMES_VAR} holds no names, so no party-name scan ran. \
             The structural rules in this file still did."
        );
        return;
    }

    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("the repository root is two levels above this crate");

    // Deliberately `Vec<String>` of locations only. No matched text, no excerpt,
    // no name — this runs in public CI with public logs.
    let mut locations = Vec::new();
    let mut scanned = 0usize;

    for (path, rel) in text_files(&root) {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        scanned += 1;
        for (lineno, line) in text.lines().enumerate() {
            let haystack = line.to_lowercase();
            if names.iter().any(|n| find_word(&haystack, n).is_some()) {
                locations.push(format!("{rel}:{}", lineno + 1));
            }
        }
    }

    assert!(scanned > 0, "scanned no files at all — the walk is broken");

    assert!(
        locations.is_empty(),
        "A private party is named in {} location(s) in this public repository:\n\n  {}\n\n\
         The matched text is deliberately not printed: these logs are public, and \
         reporting the name would publish it on the commit that tried to. Open each \
         line and remove the party reference — write the substance, drop the name.",
        locations.len(),
        locations.join("\n  ")
    );
}
