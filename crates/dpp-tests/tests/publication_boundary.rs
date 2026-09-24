//! The publication boundary: what this workspace publishes names no consumer,
//! no planning taxonomy and no commercial state.
//!
//! # Why a tripwire
//!
//! The boundary was swept three times (#343, #344, #352) and the same class of
//! violation survived each one. Every pass grepped for whatever the last one had
//! found — the repository name, then its definite nouns — and the rule it was
//! enforcing is explicit that a bare definite noun is the same violation as the
//! name. A grep scoped by the previous grep cannot see the next shape. A test
//! that runs on every change can.
//!
//! # What it reads
//!
//! The published artifact as the repository's rules define it: every crate's
//! `src` and README, the root policy documents, everything under `docs/`, and
//! the CHANGELOG's **unreleased** section. Released CHANGELOG entries are the
//! record of what shipped under each version; rewriting one publishes nothing
//! and unpublishes nothing, so they are left as history. `CLAUDE.md` is not
//! read: it is where the rule itself names what it forbids.
//!
//! Whole lines are read, not only comments. Every term below is a phrase or a
//! numbering shape that no Rust identifier spells, so code cannot bury the
//! signal — and the worst of #352's findings was in a runtime error string,
//! which a comments-only reader would have missed.
//!
//! # The list is shapes of sentence, not secrets
//!
//! An earlier private-reference scanner was parked because its denylist was
//! itself the leak: a list of forbidden private names, committed to a public
//! repository, publishes what it protects. Nothing here is like that. Every term
//! is a generic English phrase or a numbering scheme; the one real name, the
//! consumer repository's, is public.
//!
//! # Exceptions
//!
//! A legitimate use is exempted by a marker in the file itself —
//! `BOUNDARY-EXCEPTION(<class>): <reason>`, in a `//` comment or an HTML
//! comment — which exempts that file from one class and has to say why. The
//! reason is the point: it is greppable, and it cannot be written without
//! deciding that the use is legitimate. A marker that no longer exempts
//! anything fails, so an exception cannot outlive its use.
//!
//! There is no baseline of tolerated violations. Every one found when this
//! landed was reworded or given a stated exception, so anything that fails here
//! is new.

use std::fs;
use std::path::{Path, PathBuf};

const MARKER: &str = "BOUNDARY-EXCEPTION(";

/// What a term says about the publication boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Class {
    /// A definite noun, or a compound of one, naming one particular consumer —
    /// a reader who vendored a crate cannot tell which.
    Consumer,
    /// An internal planning scheme: a numbered gap, phase or chunk, or a
    /// priority tag. Meaningful only beside the plan it came from.
    Taxonomy,
    /// Commercial or vendor state: licensing arrangements, quotes, rates, lead
    /// times, contract terms.
    Commercial,
}

impl Class {
    const ALL: [Self; 3] = [Self::Consumer, Self::Taxonomy, Self::Commercial];

    fn key(self) -> &'static str {
        match self {
            Self::Consumer => "consumer",
            Self::Taxonomy => "taxonomy",
            Self::Commercial => "commercial",
        }
    }
}

/// Matched case-insensitively, starting at a word boundary.
struct Phrase {
    text: &'static str,
    /// Whether the match must also end at a word boundary. `false` lets a stem
    /// cover its inflections — `sublicen` is `sublicense`, `sublicensed` and
    /// `sublicence` at once.
    whole_word: bool,
}

const fn word(text: &'static str) -> Phrase {
    Phrase {
        text,
        whole_word: true,
    }
}

const fn stem(text: &'static str) -> Phrase {
    Phrase {
        text,
        whole_word: false,
    }
}

/// Definite nouns and compounds naming the consumer. A common noun — "a policy
/// engine", "GS1's Barcode Syntax Engine" — is permitted and matches none of
/// these, because each needs the bare definite article or the compound.
const CONSUMER: &[Phrase] = &[
    word("the engine"),
    word("the platform"),
    word("the vault"),
    word("the node's"),
    word("platform repo"),
    word("engine repo"),
    word("dpp-engine"),
    word("engine-side"),
    word("engine side"),
    word("engine-layer"),
    word("engine layer"),
    word("engine-only"),
    word("platform-side"),
    word("platform side"),
    word("platform-layer"),
    word("platform layer"),
    word("platform-only"),
    word("vault-side"),
    word("vault-only"),
];

const COMMERCIAL: &[Phrase] = &[
    stem("sublicen"),
    stem("reseller"),
    word("agreed with counsel"),
    word("per-unit rate"),
    word("lead time"),
    word("price quote"),
    word("contract terms"),
    word("under contract"),
];

/// Numbered planning terms: the word, then a space or hyphen, then a digit.
/// Case-sensitive, because the schemes are written capitalised and the
/// lower-case words are ordinary English.
const NUMBERED: &[&str] = &["Gap", "Phase", "Chunk", "chunk"];

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

fn starts_a_word(line: &str, at: usize) -> bool {
    !line[..at].chars().next_back().is_some_and(is_word_char)
}

fn ends_a_word(line: &str, at: usize) -> bool {
    !line[at..].chars().next().is_some_and(is_word_char)
}

/// Every phrase in `phrases` that occurs in `line`.
fn phrase_hits(line: &str, phrases: &[Phrase]) -> Vec<&'static str> {
    // ASCII lowering keeps byte offsets aligned with `line`.
    let lower = line.to_ascii_lowercase();
    phrases
        .iter()
        .filter(|p| {
            lower.match_indices(p.text).any(|(at, _)| {
                starts_a_word(line, at) && (!p.whole_word || ends_a_word(line, at + p.text.len()))
            })
        })
        .map(|p| p.text)
        .collect()
}

/// Every planning-taxonomy shape in `line`.
fn taxonomy_hits(line: &str) -> Vec<&'static str> {
    let mut hits = Vec::new();

    for &term in NUMBERED {
        let numbered = line.match_indices(term).any(|(at, _)| {
            let mut rest = line[at + term.len()..].chars();
            starts_a_word(line, at)
                && matches!(rest.next(), Some(' ' | '-'))
                && rest.next().is_some_and(|c| c.is_ascii_digit())
        });
        if numbered {
            hits.push(term);
        }
    }

    // `N-3`: a lettered-and-numbered tracking tag.
    if line.match_indices("N-").any(|(at, _)| {
        starts_a_word(line, at) && line[at + 2..].starts_with(|c: char| c.is_ascii_digit())
    }) {
        hits.push("N-<n>");
    }

    // `P0`–`P3` as a whole word: a priority tag.
    if line.match_indices('P').any(|(at, _)| {
        let rest = &line[at + 1..];
        starts_a_word(line, at)
            && rest.starts_with(['0', '1', '2', '3'])
            && ends_a_word(line, at + 2)
    }) {
        hits.push("P<n>");
    }

    if line.contains("R-phase") {
        hits.push("R-phase");
    }

    hits
}

/// Every term of `class` that occurs in `line`.
fn hits(class: Class, line: &str) -> Vec<&'static str> {
    match class {
        Class::Consumer => phrase_hits(line, CONSUMER),
        Class::Taxonomy => taxonomy_hits(line),
        Class::Commercial => phrase_hits(line, COMMERCIAL),
    }
}

// ─── What is read ────────────────────────────────────────────────────────────

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..")
}

/// A published file and the lines of it this gate reads.
struct Surface {
    /// Relative to the workspace root, forward slashes.
    path: String,
    /// `(line number, text)` for every line read.
    lines: Vec<(usize, String)>,
    /// The whole file, for its exception markers.
    text: String,
}

fn read(root: &Path, file: &Path) -> Surface {
    let text =
        fs::read_to_string(file).unwrap_or_else(|e| panic!("cannot read {}: {e}", file.display()));
    let path = file
        .strip_prefix(root)
        .unwrap_or(file)
        .display()
        .to_string()
        .replace('\\', "/");
    let lines = text
        .lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.to_owned()))
        .collect();
    Surface { path, lines, text }
}

fn walk(dir: &Path, extension: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, extension, out);
        } else if path.extension().is_some_and(|e| e == extension) {
            out.push(path);
        }
    }
}

/// The published surface. Crates are discovered, not listed, so a new crate
/// cannot go unread.
fn published_surface() -> Vec<Surface> {
    let root = workspace_root();
    let mut files = Vec::new();

    let mut crates = 0;
    for entry in fs::read_dir(root.join("crates"))
        .expect("crates/")
        .flatten()
    {
        let dir = entry.path();
        if dir.join("Cargo.toml").is_file() {
            crates += 1;
            walk(&dir.join("src"), "rs", &mut files);
            let readme = dir.join("README.md");
            if readme.is_file() {
                files.push(readme);
            }
        }
    }
    assert!(crates > 5, "found {crates} crates — has the layout moved?");

    for policy in [
        "README.md",
        "SECURITY.md",
        "GOVERNANCE.md",
        "CONTRIBUTING.md",
    ] {
        files.push(root.join(policy));
    }
    walk(&root.join("docs"), "md", &mut files);

    let mut surface: Vec<Surface> = files.iter().map(|f| read(&root, f)).collect();
    surface.push(unreleased_changelog(&root));
    surface
}

/// The CHANGELOG up to the first released version's heading.
fn unreleased_changelog(root: &Path) -> Surface {
    let mut changelog = read(root, &root.join("CHANGELOG.md"));
    let released = changelog
        .lines
        .iter()
        .position(|(_, l)| l.starts_with("## [") && !l.starts_with("## [Unreleased]"))
        .expect("the CHANGELOG has a released version");
    changelog.lines.truncate(released);
    changelog
}

// ─── Exceptions ──────────────────────────────────────────────────────────────

/// What follows the marker, when `line` is one.
///
/// A marker has to **open** a `//` or HTML comment. Prose that mentions the
/// syntax mid-sentence — a CHANGELOG entry or a doc explaining it — is not
/// one, and must not be read as an exemption or refused as a malformed one.
fn marker(line: &str) -> Option<&str> {
    let line = line.trim_start();
    ["// ", "<!-- "]
        .into_iter()
        .find_map(|opener| line.strip_prefix(opener)?.strip_prefix(MARKER))
}

/// The classes a file's markers exempt it from, or why a marker is malformed.
fn exemptions(surface: &Surface) -> Result<Vec<Class>, String> {
    let mut exempt = Vec::new();
    for line in surface.text.lines().filter_map(marker) {
        let Some((key, reason)) = line.split_once("):") else {
            return Err(format!(
                "{}: a marker must read `{MARKER}<class>): <reason>`",
                surface.path
            ));
        };
        let Some(class) = Class::ALL.into_iter().find(|c| c.key() == key) else {
            return Err(format!(
                "{}: unknown class `{key}` in a marker",
                surface.path
            ));
        };
        let reason = reason.trim().trim_end_matches("-->").trim();
        if reason.is_empty() {
            return Err(format!(
                "{}: a `{key}` exception gives no reason",
                surface.path
            ));
        }
        exempt.push(class);
    }
    Ok(exempt)
}

// ─── The gates ───────────────────────────────────────────────────────────────

#[test]
fn the_published_surface_crosses_no_boundary() {
    let mut violations = Vec::new();
    for surface in published_surface() {
        let exempt = exemptions(&surface).unwrap_or_else(|e| panic!("{e}"));
        for (number, line) in &surface.lines {
            if marker(line).is_some() {
                continue;
            }
            for class in Class::ALL.into_iter().filter(|c| !exempt.contains(c)) {
                for term in hits(class, line) {
                    violations.push(format!(
                        "{}:{number}: {} `{term}` — {}",
                        surface.path,
                        class.key(),
                        line.trim()
                    ));
                }
            }
        }
    }
    assert!(
        violations.is_empty(),
        "published prose crosses the publication boundary — reword it, or, if the use is \
         legitimate, add `{MARKER}<class>): <reason>` to the file:\n{}",
        violations.join("\n")
    );
}

/// An exception for a class the file no longer uses is stale, and a stale
/// exception is a hole waiting for the next violation to fall into.
#[test]
fn every_exception_is_still_needed() {
    let mut stale = Vec::new();
    for surface in published_surface() {
        let exempt = exemptions(&surface).unwrap_or_else(|e| panic!("{e}"));
        for class in exempt {
            let used = surface
                .lines
                .iter()
                .filter(|(_, l)| marker(l).is_none())
                .any(|(_, l)| !hits(class, l).is_empty());
            if !used {
                stale.push(format!("{} — `{}`", surface.path, class.key()));
            }
        }
    }
    assert!(
        stale.is_empty(),
        "these exceptions exempt nothing any more; remove them:\n{}",
        stale.join("\n")
    );
}

// ─── The matcher, watched to fail ────────────────────────────────────────────

/// Each class catches the shapes #352 found by hand.
#[test]
fn each_class_catches_what_it_is_for() {
    for (class, line) in [
        (Class::Consumer, "/// Resolved by the engine at runtime."),
        (Class::Consumer, "//! The Engine's verifier walks the tree."),
        (
            Class::Consumer,
            "enforced platform-side, because core cannot",
        ),
        (Class::Consumer, "see `dpp-engine` for the adapter"),
        (Class::Consumer, "the node's `KeyStore`"),
        (Class::Taxonomy, "// closes crypto Gap 5"),
        (Class::Taxonomy, "deferred to Phase 2 of the roadmap"),
        (Class::Taxonomy, "a Phase-1 item"),
        (Class::Taxonomy, "tracked as N-12"),
        (Class::Taxonomy, "priority P0"),
        (Class::Taxonomy, "review chunk 4"),
        (
            Class::Commercial,
            "\"Signed ecoinvent / EF dataset reseller sublicense\"",
        ),
        (
            Class::Commercial,
            "Legal warranty scope agreed with counsel",
        ),
        (Class::Commercial, "the vendor's lead time is six weeks"),
    ] {
        assert!(!hits(class, line).is_empty(), "{:?} missed: {line}", class);
    }
}

/// The legitimate uses #352 kept must not trip it.
#[test]
fn the_uses_worth_keeping_pass() {
    for line in [
        "What checks it is GS1's own Barcode Syntax Engine, and GS1's engine agrees.",
        "the GS1 Barcode Syntax Engine",
        "IDTA's test engine",
        "an access filter engine, a policy engine or a rules engine",
        "use base64::Engine as _;",
        "base64::engine::general_purpose::URL_SAFE_NO_PAD",
        "the engineering trade-off",
        "wasmtime sandbox (WASI Preview 1 syscall interface)",
        "impl RecycledContentRuleset for Art8Phase1Ruleset",
        "the host/guest contract",
        "a `typ` containing a quote",
        "### G7 — A lineage edge was asserted",
        "an EN 18219 identifier, not a UUID",
    ] {
        for class in Class::ALL {
            assert!(
                hits(class, line).is_empty(),
                "{:?} fired on a legitimate use: {line}",
                class
            );
        }
    }
}

/// A marker has to name a class it knows and give a reason.
#[test]
fn a_marker_must_name_a_class_and_a_reason() {
    let surface = |text: &str| Surface {
        path: "x.md".into(),
        lines: Vec::new(),
        text: text.into(),
    };
    assert_eq!(
        exemptions(&surface(
            "<!-- BOUNDARY-EXCEPTION(taxonomy): the Regulation's own phases -->"
        )),
        Ok(vec![Class::Taxonomy])
    );
    assert!(exemptions(&surface("// BOUNDARY-EXCEPTION(taxonomy):")).is_err());
    assert!(exemptions(&surface("<!-- BOUNDARY-EXCEPTION(taxonomy): -->")).is_err());
    assert!(exemptions(&surface("// BOUNDARY-EXCEPTION(everything): because")).is_err());
    assert!(exemptions(&surface("// BOUNDARY-EXCEPTION(taxonomy) because")).is_err());

    // Prose explaining the syntax is not a marker, so it neither exempts the
    // file nor fails as a malformed one.
    assert_eq!(
        exemptions(&surface(
            "  exempted by `BOUNDARY-EXCEPTION(<class>): <reason>`, which must"
        )),
        Ok(Vec::new())
    );
}
