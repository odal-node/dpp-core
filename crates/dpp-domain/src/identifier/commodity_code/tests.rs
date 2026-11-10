//! Parsing and validity of a CN commodity code.

use super::{CommodityCode, DeclaredCommodityCode};

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

/// The code is its own wire form, valid on the way in as on the way out.
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

/// Reading calls `parse`, as `Gtin` does: what it refuses to build it refuses to
/// read. `"abc"` used to deserialise and hold `abc`.
#[test]
fn a_string_that_is_not_a_code_does_not_deserialise() {
    for wrong in ["abc", "", "8507", "85076000X", "8507 60 00"] {
        let json = serde_json::to_string(wrong).unwrap();
        assert!(
            serde_json::from_str::<CommodityCode>(&json).is_err(),
            "{wrong:?} was read as a code"
        );
    }
}

/// A declared code is kept as written, valid or not, and writes back as the same
/// string, because a document that declares one is read after the fact and a fetched
/// one cannot be rewritten.
#[test]
fn a_declared_code_keeps_any_string_verbatim() {
    for written in ["85076000", "abc", "", " 85076000 ", "8507 60 00"] {
        let json = serde_json::to_string(written).unwrap();
        let declared: DeclaredCommodityCode = serde_json::from_str(&json).unwrap();

        assert_eq!(declared.as_str(), written);
        assert_eq!(serde_json::to_string(&declared).unwrap(), json);
    }
}

/// Whether a declared code is a valid one is a separate question, answered when it
/// is asked, and `parse`'s answer: padding is trimmed in the code it returns, and
/// the declared string stays as written.
#[test]
fn a_declared_code_says_whether_it_is_valid_when_asked() {
    assert!(DeclaredCommodityCode::new("85076000").validated().is_ok());
    assert!(DeclaredCommodityCode::new("abc").validated().is_err());

    let padded = DeclaredCommodityCode::new(" 85076000 ");
    assert_eq!(padded.validated().unwrap().as_str(), "85076000");
    assert_eq!(padded.as_str(), " 85076000 ");
}

/// A valid code and the declaration of it are the same JSON, so a document is the
/// same whichever it was written from.
#[test]
fn a_declared_code_and_a_code_share_a_wire_form() {
    let code = CommodityCode::parse("85076000").unwrap();
    let declared = DeclaredCommodityCode::from(code.clone());

    assert_eq!(
        serde_json::to_string(&code).unwrap(),
        serde_json::to_string(&declared).unwrap()
    );
    assert_eq!(declared.to_string(), "85076000");
}
