//! Tests of the `syntax_dictionary` module.

use super::super::super::component::{CharKind, Component};
use super::super::*;
use std::collections::HashMap;

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
    assert!(batch.cset_82, "10 is `X..20`");

    let serial = ai_spec("21").expect("21 present");
    assert_eq!((serial.min_len, serial.max_len), (1, 20));
    assert!(serial.cset_82, "21 is `X..20`");
    // Numeric, and mixed numeric-then-`X`: neither is a whole-value CSET 82.
    assert!(!gtin.cset_82);
    assert!(!ai_spec("253").expect("253 present").cset_82);

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

/// GS1 defines sixteen Digital Link primary keys, and the URI parser takes
/// its set from this flag rather than a list of its own. The set is pinned
/// here so a dictionary update that adds or drops a key shows up as a
/// failing test instead of quietly changing which URIs parse.
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
            "00", "01", "253", "255", "401", "402", "414", "415", "417", "8003", "8004", "8006",
            "8010", "8013", "8017", "8018"
        ]
    );
}

/// The components the value check reads are the entry's own, in order, with
/// the linters it names. Pinned against entries read out of the vendored file.
#[test]
fn reads_each_components_set_length_and_linters() {
    let grai = ai_spec("8003").expect("8003 present");
    assert_eq!(
        grai.components,
        vec![
            Component {
                kind: CharKind::Numeric,
                min_len: 1,
                max_len: 1,
                optional: false,
                linters: vec!["zero".into()],
            },
            Component {
                kind: CharKind::Numeric,
                min_len: 13,
                max_len: 13,
                optional: false,
                linters: vec!["csum".into(), "gcppos1".into()],
            },
            Component {
                kind: CharKind::Cset82,
                min_len: 1,
                max_len: 16,
                optional: true,
                linters: vec![],
            },
        ]
    );

    let cpid = ai_spec("8010").expect("8010 present");
    assert_eq!(cpid.components[0].kind, CharKind::Cset39);
    assert_eq!((cpid.min_len, cpid.max_len), (1, 30));

    let uic = ai_spec("7040").expect("7040 present");
    let kinds: Vec<CharKind> = uic.components.iter().map(|c| c.kind).collect();
    assert_eq!(
        kinds,
        [
            CharKind::Numeric,
            CharKind::Cset82,
            CharKind::Cset82,
            CharKind::Cset82
        ]
    );
}

/// Every AI the dictionary describes has components that add up to the
/// lengths it reports, so the two summaries can never disagree with the
/// whole they summarise.
#[test]
fn components_account_for_the_reported_lengths() {
    for spec in dictionary().values() {
        let min: usize = spec
            .components
            .iter()
            .filter(|c| !c.optional)
            .map(|c| c.min_len)
            .sum();
        let max: usize = spec.components.iter().map(|c| c.max_len).sum();
        assert_eq!(
            (spec.min_len, spec.max_len),
            (min, max),
            "AI {} components disagree with its lengths",
            spec.ai
        );
    }
}

/// Only AI 415 depends on one of its own qualifiers: it declares
/// `req=8020 dlpkey=8020`. The rule is read from those attributes, so a key
/// whose requisites are something else, or are several, is not made
/// dependent by it.
#[test]
fn only_the_pay_to_gln_requires_a_qualifier() {
    assert_eq!(required_qualifier("415"), Some("8020"));
    for spec in dictionary().values().filter(|s| s.dl_primary_key) {
        if spec.ai != "415" {
            assert_eq!(required_qualifier(&spec.ai), None, "AI {}", spec.ai);
        }
    }
    assert_eq!(
        required_qualifier("10"),
        None,
        "a qualifier is no primary key"
    );
}
