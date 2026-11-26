//! The GS1 syntax dictionary and the specification of each application identifier.

use std::collections::HashMap;
use std::sync::OnceLock;

use super::super::component::{CharKind, Component};

/// GS1's published dictionary, vendored at a tagged release.
pub(super) const DICTIONARY: &str = include_str!("../../../data/gs1-syntax-dictionary.txt");

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
    /// Every component is type `X`, so the whole value must stay inside GS1
    /// CSET 82 — true of AI 10 and AI 21.
    ///
    /// Whole-value only. An AI mixing types (`N14` followed by `X..16`) needs
    /// its value split by component before any one character set applies, and
    /// that split is not performed here.
    pub cset_82: bool,
    /// Human-readable title from the trailing comment, e.g. `"GTIN"`.
    pub title: String,
    /// The specification's components, in order. `N14,csum` is one component;
    /// `N13,csum [X..17]` is two, the second optional.
    ///
    /// What `min_len`, `max_len` and `cset_82` summarise, kept whole so a value
    /// can be held to each component's own length and character set, which a
    /// whole-value summary cannot do for an AI that mixes types.
    pub components: Vec<Component>,
    /// The tokens of the entry's `req=` attribute, as written. For a primary
    /// key whose only requisite is one of its own qualifiers, that qualifier is
    /// mandatory: AI 415 declares `req=8020 dlpkey=8020`.
    pub requisites: Vec<String>,
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

/// The qualifier a primary key cannot appear without, if it has one.
///
/// The dictionary says so in two places that agree: AI 415 declares `req=8020`
/// and `dlpkey=8020`, so its only requisite is its own qualifier. Read from the
/// data rather than written here, so a key GS1 makes dependent later is picked
/// up with the dictionary.
#[must_use]
pub fn required_qualifier(primary_key: &str) -> Option<&'static str> {
    let spec = ai_spec(primary_key)?;
    let [only] = spec.requisites.as_slice() else {
        return None;
    };
    spec.dl_qualifiers
        .iter()
        .flatten()
        .find(|qualifier| *qualifier == only)
        .map(String::as_str)
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
/// recognising those AIs, not take the process down. `every_entry_parses` is
/// what turns such a skip into a failing test.
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
        let mut cset_82 = true;
        let mut dl_primary_key = false;
        let mut dl_qualifiers: Vec<Vec<String>> = Vec::new();
        let mut components: Vec<Component> = Vec::new();
        let mut requisites: Vec<String> = Vec::new();
        for field in &rest[spec_start..] {
            if field.starts_with(COMPONENT_START) {
                let optional = field.starts_with('[');
                let component = field.trim_start_matches('[').trim_end_matches(']');
                cset_82 &= component.starts_with('X');
                if let Some((lo, hi)) = component_len(component) {
                    if !optional {
                        min_len += lo;
                    }
                    max_len += hi;
                    if let Some(kind) = char_kind(component) {
                        components.push(Component {
                            kind,
                            min_len: lo,
                            max_len: hi,
                            optional,
                            linters: component.split(',').skip(1).map(str::to_owned).collect(),
                        });
                    }
                }
            } else if let Some(value) = field.strip_prefix("req=") {
                requisites = value.split(',').map(str::to_owned).collect();
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
                    cset_82,
                    title: title.clone(),
                    components: components.clone(),
                    requisites: requisites.clone(),
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

/// The character set a component's type letter names.
fn char_kind(component: &str) -> Option<CharKind> {
    match component.chars().next()? {
        'N' => Some(CharKind::Numeric),
        'X' => Some(CharKind::Cset82),
        'Y' => Some(CharKind::Cset39),
        'Z' => Some(CharKind::Cset64),
        _ => None,
    }
}

/// Expand `3100-3105` into each AI it covers; a solitary AI yields itself.
///
/// Ranges are always numeric and equal-width in the dictionary, so the bounds
/// are parsed as integers and re-padded to the original width.
pub(super) fn expand_ai_field(field: &str) -> Vec<String> {
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
