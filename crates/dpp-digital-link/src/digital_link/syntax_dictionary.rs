//! The GS1 Barcode Syntax Dictionary, parsed into an Application Identifier table.
//!
//! The table is **derived from GS1's published dictionary**, never hand-written.
//! Its central fact is per-AI: whether the AI is *pre-defined length* — GS1's
//! `*` flag, meaning no FNC1 separator follows it — or variable length, running
//! to the next separator. Get one wrong and a scanned element string is
//! silently truncated or over-read, producing an identifier that still looks
//! plausible.
//!
//! Provenance, licence and update procedure: `data/README.md`.
//!
//! ## Entry syntax
//!
//! ```text
//! AIs  [Flags]  Specification  [Attributes...]  [# Title]
//!
//! 01         *?  N14,csum,gcppos2   ex=255,37 dlpkey=22,10,21|235   # GTIN
//! 10          ?  X..20              req=01,02,03,8006,8026          # BATCH/LOT
//! 3100-3105  *?  N6                 req=01,02 ex=310n               # NET WEIGHT (kg)
//! 253         ?  N13,csum,gcppos1 [X..17]   dlpkey                  # GDTI
//! ```
//!
//! - **AIs** — one AI, or an inclusive range (`3100-3105`), expanded here.
//! - **Flags** — `*` pre-defined length; `?` permitted as a Digital Link data
//!   attribute. Other flag characters are reserved and ignored.
//! - **Specification** — whitespace-separated components, each `Type[,linter…]`
//!   where the type is `N`/`X`/`Y`/`Z` plus `5` (exactly) or `..20` (up to).
//!   `[…]` marks an optional component. Only the final component may vary.
//! - **Attributes** — `dlpkey` marks a Digital Link primary key.
//!
//! ## What this does not do
//!
//! The dictionary names a *linter* per component (`csum`, `gcppos2`, …) whose
//! reference implementations are a separate GS1 resource that is not vendored.
//! This module reads lengths and flags. Content validation beyond the check
//! digit this crate already implements is not performed, and nothing here
//! supports a claim of full GS1 validation.

use std::collections::HashMap;
use std::sync::OnceLock;

/// GS1's published dictionary, vendored at a tagged release.
const DICTIONARY: &str = include_str!("../../data/gs1-syntax-dictionary.txt");

/// Characters a specification component may begin with: a type letter, or the
/// bracket that opens an optional component. Anything else at that position is
/// a flags field or an attribute, not a component.
const COMPONENT_START: [char; 5] = ['N', 'X', 'Y', 'Z', '['];

/// What the dictionary says about one Application Identifier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AiSpec {
    /// The AI itself, e.g. `"01"`, `"3103"`.
    pub ai: String,
    /// GS1's `*` flag: the value has a pre-defined length, so no FNC1
    /// separator follows it in an element string.
    pub predefined_length: bool,
    /// GS1's `?` flag: permitted as a Digital Link data attribute.
    pub dl_attribute: bool,
    /// `dlpkey`: this AI may open a Digital Link path.
    pub dl_primary_key: bool,
    /// The qualifier sequences this primary key accepts, from `dlpkey=`.
    ///
    /// Each inner `Vec` is one **alternative** sequence, in the order the
    /// qualifiers must appear. GS1 writes alternatives with `|`, so AI 01's
    /// `dlpkey=22,10,21|235` parses to `[[22, 10, 21], [235]]` — a path may
    /// use qualifiers from one sequence or the other, never a mix.
    ///
    /// Empty when the AI is not a primary key, or is a bare `dlpkey` that
    /// accepts no qualifiers at all.
    pub dl_qualifiers: Vec<Vec<String>>,
    /// Shortest legal value, summing the mandatory components.
    pub min_len: usize,
    /// Longest legal value, summing every component including optional ones.
    pub max_len: usize,
    /// Human-readable title from the trailing comment, e.g. `"GTIN"`.
    pub title: String,
}

/// The parsed dictionary, keyed by AI. Built once on first use.
pub fn dictionary() -> &'static HashMap<String, AiSpec> {
    static TABLE: OnceLock<HashMap<String, AiSpec>> = OnceLock::new();
    TABLE.get_or_init(|| parse_dictionary(DICTIONARY))
}

/// Look up one AI.
#[must_use]
pub fn ai_spec(ai: &str) -> Option<&'static AiSpec> {
    dictionary().get(ai)
}

/// Where `qualifier` sits among `primary_key`'s qualifier sequences.
///
/// Returns `(sequence, position)` — the index of the alternative sequence it
/// belongs to, and its 1-based position within that sequence.
///
/// Both halves are load-bearing. Position orders qualifiers within a sequence;
/// the sequence index is what makes GS1's `|` alternatives real, because two
/// qualifiers drawn from **different** sequences may not appear in one path.
/// AI 01 declares `dlpkey=22,10,21|235`, so `/21/…/235/…` is not a longer
/// version of `/21/…` — it mixes two alternatives, and GS1 defines no such
/// link.
#[must_use]
pub fn qualifier_position(primary_key: &str, qualifier: &str) -> Option<(usize, u8)> {
    let spec = ai_spec(primary_key)?;
    spec.dl_qualifiers
        .iter()
        .enumerate()
        .find_map(|(seq, ais)| {
            ais.iter()
                .position(|a| a == qualifier)
                .map(|pos| (seq, u8::try_from(pos + 1).unwrap_or(u8::MAX)))
        })
}

/// How long an AI beginning with `prefix` (its first two characters) is.
///
/// This is GS1's own rule for splitting an element string: the leading two
/// digits determine how many digits the AI occupies. It is deterministic, which
/// longest-match is not — with longest-match, an AI followed by a value whose
/// first digits happen to complete a longer AI would be mis-split, shifting
/// every value after it.
///
/// The mapping is derived from the dictionary, and
/// `prefix_determines_ai_length` asserts the property it relies on.
pub fn ai_len_for_prefix(prefix: &str) -> Option<usize> {
    static PREFIXES: OnceLock<HashMap<String, usize>> = OnceLock::new();
    PREFIXES
        .get_or_init(|| {
            let mut map = HashMap::new();
            for ai in dictionary().keys() {
                if let Some(p) = ai.get(..2) {
                    map.insert(p.to_owned(), ai.len());
                }
            }
            map
        })
        .get(prefix)
        .copied()
}

/// Parse the dictionary text into a table, skipping comments and blank lines.
///
/// Malformed entries are skipped rather than panicking: a future release
/// introducing syntax this parser does not know should degrade to not
/// recognising those AIs, not take the process down. `dictionary_covers_every_entry`
/// is what turns such a skip into a failing test.
fn parse_dictionary(text: &str) -> HashMap<String, AiSpec> {
    let mut table = HashMap::new();

    for line in text.lines() {
        let line = line.trim_end();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Split off the trailing "# Title".
        let (body, title) = match line.split_once('#') {
            Some((body, title)) => (body, title.trim().to_owned()),
            None => (line, String::new()),
        };

        let mut fields = body.split_whitespace();
        let Some(ai_field) = fields.next() else {
            continue;
        };
        let rest: Vec<&str> = fields.collect();
        if rest.is_empty() {
            continue;
        }

        // Flags are optional and are drawn from a punctuation set, so a field
        // that starts with a type letter is already the specification.
        let (flags, spec_start) = if rest[0].starts_with(COMPONENT_START) {
            ("", 0)
        } else {
            (rest[0], 1)
        };

        let predefined_length = flags.contains('*');
        let dl_attribute = flags.contains('?');

        // The specification runs until the first attribute. Attributes are
        // `key=value` pairs or bare keys; components always start with a type
        // letter or an opening bracket.
        let mut min_len = 0usize;
        let mut max_len = 0usize;
        let mut dl_primary_key = false;
        let mut dl_qualifiers: Vec<Vec<String>> = Vec::new();
        for field in &rest[spec_start..] {
            if field.starts_with(COMPONENT_START) {
                let optional = field.starts_with('[');
                let component = field.trim_start_matches('[').trim_end_matches(']');
                if let Some((lo, hi)) = component_len(component) {
                    if !optional {
                        min_len += lo;
                    }
                    max_len += hi;
                }
            } else if *field == "dlpkey" {
                dl_primary_key = true;
            } else if let Some(value) = field.strip_prefix("dlpkey=") {
                dl_primary_key = true;
                // `22,10,21|235` — alternative sequences separated by `|`,
                // each an ordered, optional qualifier list.
                dl_qualifiers = value
                    .split('|')
                    .map(|seq| seq.split(',').map(str::to_owned).collect())
                    .collect();
            }
        }

        if max_len == 0 {
            continue;
        }

        for ai in expand_ai_field(ai_field) {
            table.insert(
                ai.clone(),
                AiSpec {
                    ai,
                    predefined_length,
                    dl_attribute,
                    dl_primary_key,
                    dl_qualifiers: dl_qualifiers.clone(),
                    min_len,
                    max_len,
                    title: title.clone(),
                },
            );
        }
    }

    table
}

/// Minimum and maximum length of one specification component.
///
/// `N14` is exactly 14; `X..20` is 1 to 20. The type letter and any trailing
/// `,linter` names are not length information and are discarded.
fn component_len(component: &str) -> Option<(usize, usize)> {
    let spec = component.split(',').next()?;
    let digits = spec.get(1..)?;
    match digits.strip_prefix("..") {
        Some(max) => {
            let max = max.parse().ok()?;
            Some((1, max))
        }
        None => {
            let exact = digits.parse().ok()?;
            Some((exact, exact))
        }
    }
}

/// Expand `3100-3105` into each AI it covers; a solitary AI yields itself.
///
/// Ranges are always numeric and equal-width in the dictionary, so the bounds
/// are parsed as integers and re-padded to the original width.
fn expand_ai_field(field: &str) -> Vec<String> {
    let Some((lo, hi)) = field.split_once('-') else {
        return vec![field.to_owned()];
    };
    let width = lo.len();
    let (Ok(lo), Ok(hi)) = (lo.parse::<u32>(), hi.parse::<u32>()) else {
        return vec![field.to_owned()];
    };
    if hi < lo {
        return Vec::new();
    }
    (lo..=hi).map(|n| format!("{n:0width$}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every non-comment line must yield at least one AI. A line the parser
    /// skips is an AI we would silently fail to recognise, so a dictionary
    /// update introducing unfamiliar syntax fails here rather than in the field.
    #[test]
    fn every_entry_parses() {
        let expected: Vec<&str> = DICTIONARY
            .lines()
            .map(str::trim_end)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        assert_eq!(expected.len(), 224, "vendored dictionary changed size");

        for line in expected {
            let field = line.split_whitespace().next().expect("AI field");
            for ai in expand_ai_field(field) {
                assert!(
                    dictionary().contains_key(&ai),
                    "AI {ai} (from `{line}`) was skipped by the parser"
                );
            }
        }
    }

    /// Spot-checks against entries read directly out of the vendored file.
    /// These pin the two properties the element-string parser depends on.
    #[test]
    fn reads_lengths_and_the_predefined_length_flag() {
        let gtin = ai_spec("01").expect("01 present");
        assert!(gtin.predefined_length, "01 is `*` in the dictionary");
        assert_eq!((gtin.min_len, gtin.max_len), (14, 14));
        assert!(gtin.dl_primary_key);
        assert_eq!(gtin.title, "GTIN");

        let batch = ai_spec("10").expect("10 present");
        assert!(!batch.predefined_length, "10 is variable length");
        assert_eq!((batch.min_len, batch.max_len), (1, 20));
        assert!(!batch.dl_primary_key);

        // Optional trailing component widens the maximum but not the minimum.
        let gdti = ai_spec("253").expect("253 present");
        assert_eq!((gdti.min_len, gdti.max_len), (13, 30));
        assert!(gdti.dl_primary_key);
    }

    /// The invariant the element-string parser rests on: every AI sharing a
    /// two-digit prefix has the same length, so those two digits alone say how
    /// many to consume. Verified across all 541 AIs the dictionary expands to.
    ///
    /// If a future GS1 release breaks this, the parser's splitting rule is no
    /// longer sound and this fails rather than mis-splitting scanned data.
    #[test]
    fn prefix_determines_ai_length() {
        assert_eq!(dictionary().len(), 541, "expanded AI count changed");

        let mut by_prefix: HashMap<&str, usize> = HashMap::new();
        for ai in dictionary().keys() {
            let prefix = &ai[..2];
            if let Some(seen) = by_prefix.insert(prefix, ai.len()) {
                assert_eq!(
                    seen,
                    ai.len(),
                    "prefix {prefix} covers AIs of differing lengths — \
                     element-string splitting is no longer deterministic"
                );
            }
        }

        assert_eq!(ai_len_for_prefix("01"), Some(2));
        assert_eq!(ai_len_for_prefix("31"), Some(4));
        assert_eq!(ai_len_for_prefix("80"), Some(4));
        // `99` *is* assigned — it falls inside the `91-99` INTERNAL range — so
        // an unassigned prefix is needed to test the negative case.
        assert_eq!(ai_len_for_prefix("99"), Some(2));
        assert_eq!(ai_len_for_prefix("05"), None);
    }

    /// Ranges expand to every AI they cover, at the original width.
    #[test]
    fn expands_ai_ranges() {
        assert_eq!(expand_ai_field("01"), vec!["01"]);
        assert_eq!(
            expand_ai_field("3100-3105"),
            vec!["3100", "3101", "3102", "3103", "3104", "3105"]
        );
        let net_weight = ai_spec("3103").expect("3103 comes from a range");
        assert!(net_weight.predefined_length);
        assert_eq!((net_weight.min_len, net_weight.max_len), (6, 6));
    }

    /// GS1 defines sixteen Digital Link primary keys. This crate's URI parser
    /// accepts only `01`; the set is pinned here so that gap stays measured
    /// rather than assumed, and so a dictionary update that adds a key shows up
    /// as a failing test instead of passing unnoticed.
    #[test]
    fn records_every_digital_link_primary_key() {
        let mut keys: Vec<&str> = dictionary()
            .values()
            .filter(|s| s.dl_primary_key)
            .map(|s| s.ai.as_str())
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "00", "01", "253", "255", "401", "402", "414", "415", "417", "8003", "8004",
                "8006", "8010", "8013", "8017", "8018"
            ]
        );
    }
}
