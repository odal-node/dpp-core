//! Parsing and validity of a CN commodity code.

use super::commodity_code::*;

#[test]
fn the_three_tariff_levels_parse() {
    // HS-6 subheading, CN-8, TARIC-10 — lithium-ion accumulators.
    for code in ["850760", "85076000", "8507600090"] {
        let parsed = CommodityCode::parse(code).expect("must parse");
        assert_eq!(parsed.as_str(), code);
        assert_eq!(parsed.hs_subheading(), "850760");
    }
}

#[test]
fn surrounding_whitespace_is_trimmed() {
    assert_eq!(
        CommodityCode::parse("  85076000 ").unwrap().as_str(),
        "85076000"
    );
}

/// Separators are refused rather than stripped: compacting `"8507 60 00"`
/// would turn a mistyped code into a different valid tariff heading.
#[test]
fn separators_are_refused_not_stripped() {
    for code in ["8507 60 00", "8507.60.00", "8507-60-00"] {
        assert!(
            CommodityCode::parse(code).is_err(),
            "{code} must be refused rather than compacted"
        );
    }
}

#[test]
fn wrong_lengths_are_refused() {
    // 4 (heading), 7, 9 and 12 digits are not classification levels.
    for code in ["8507", "8507600", "850760009", "850760009012", ""] {
        assert!(
            CommodityCode::parse(code).is_err(),
            "{code} must be refused"
        );
    }
}

#[test]
fn round_trips_as_a_bare_json_string() {
    let code = CommodityCode::parse("85076000").unwrap();
    let json = serde_json::to_string(&code).unwrap();
    assert_eq!(json, "\"85076000\"");
    assert_eq!(
        serde_json::from_str::<CommodityCode>(&json).unwrap(),
        code,
        "serde(transparent): the wire form is the code itself"
    );
}
