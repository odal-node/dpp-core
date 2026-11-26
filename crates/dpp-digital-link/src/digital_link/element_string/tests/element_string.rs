//! Tests of the `element_string` module.

use super::super::super::error::DigitalLinkError;
use super::super::super::link::DigitalLink;
use super::super::*;

/// A valid GTIN-14 used throughout; its check digit is correct.
const GTIN: &str = "09506000134352";

#[test]
fn reads_a_predefined_length_ai_without_a_separator() {
    let parsed = ElementString::parse(&format!("01{GTIN}")).expect("parses");
    assert_eq!(parsed.gtin.map(|g| g.to_string()), Some(GTIN.to_owned()));
    assert!(parsed.other.is_empty());
}

/// The realistic scan: symbology identifier, a pre-defined-length GTIN with
/// no separator after it, a variable serial ended by FNC1, and a variable
/// batch ended by the input.
#[test]
fn reads_a_full_scan() {
    let parsed =
        ElementString::parse(&format!("]d201{GTIN}2112345\u{1D}10BATCH01")).expect("parses");

    assert_eq!(parsed.symbology.as_deref(), Some("]d2"));
    assert_eq!(parsed.gtin.map(|g| g.to_string()), Some(GTIN.to_owned()));
    assert_eq!(parsed.serial.as_deref(), Some("12345"));
    assert_eq!(parsed.batch.as_deref(), Some("BATCH01"));
}

/// The whole point of reading lengths from GS1's dictionary. `21` is
/// variable, so its value runs to the separator; a parser that treated it as
/// fixed would truncate the serial and read the remainder as an AI.
#[test]
fn a_variable_value_runs_to_the_separator_not_a_fixed_width() {
    let parsed =
        ElementString::parse(&format!("01{GTIN}21SN-0000000001\u{1D}10B1")).expect("parses");
    assert_eq!(parsed.serial.as_deref(), Some("SN-0000000001"));
    assert_eq!(parsed.batch.as_deref(), Some("B1"));
}

/// An AI this crate does not model as a typed field is still returned.
/// Silently dropping data read off a physical product is the dangerous
/// failure, not the noisy one.
#[test]
fn keeps_unmodelled_ais_rather_than_dropping_them() {
    // 17 is a pre-defined-length date (N6), so no separator follows it.
    let parsed = ElementString::parse(&format!("01{GTIN}17260821")).expect("parses");
    assert_eq!(parsed.other, vec![("17".to_owned(), "260821".to_owned())]);
}

/// Four-digit AIs are read as four digits, from the same prefix rule.
#[test]
fn reads_a_four_digit_ai() {
    let parsed = ElementString::parse(&format!("01{GTIN}3103000123")).expect("parses");
    assert_eq!(parsed.other, vec![("3103".to_owned(), "000123".to_owned())]);
}

#[test]
fn rejects_an_unassigned_ai() {
    let err = ElementString::parse("05SOMETHING").expect_err("05 is unassigned");
    assert!(
        matches!(err, DigitalLinkError::UnknownApplicationIdentifier(_)),
        "got {err:?}"
    );
}

#[test]
fn rejects_a_truncated_predefined_length_value() {
    let err = ElementString::parse("010950600013").expect_err("GTIN is short");
    assert!(
        matches!(err, DigitalLinkError::TrailingUnpairedSegment(_)),
        "got {err:?}"
    );
}

#[test]
fn rejects_an_over_long_variable_value() {
    // AI 21 permits at most 20 characters.
    let err = ElementString::parse(&format!("01{GTIN}21{}", "X".repeat(21)))
        .expect_err("serial too long");
    assert!(
        matches!(err, DigitalLinkError::ValueTooLong { max_len: 20, .. }),
        "got {err:?}"
    );
}

/// A bad check digit is caught here, not carried forward.
#[test]
fn validates_the_gtin_check_digit() {
    let err = ElementString::parse("0109506000134353").expect_err("check digit is wrong");
    assert!(
        matches!(err, DigitalLinkError::InvalidGtinCheckDigit { .. }),
        "got {err:?}"
    );
}

/// The conversion callers actually want, and it round-trips through the
/// URI parser this crate already had.
#[test]
fn converts_to_a_digital_link_and_round_trips() {
    let parsed =
        ElementString::parse(&format!("]d201{GTIN}2112345\u{1D}10BATCH01")).expect("parses");
    let link = parsed
        .to_digital_link("https://id.example.com")
        .expect("has a GTIN");

    let uri = link.build();
    let reparsed = DigitalLink::parse(&uri).expect("round-trips");
    assert_eq!(reparsed.gtin().unwrap().to_string(), GTIN);
    assert_eq!(reparsed.serial(), Some("12345"));
    assert_eq!(reparsed.batch(), Some("BATCH01"));
}

/// GS1 defines sixteen Digital Link primary keys and this crate builds
/// links only on GTIN. An element string keyed on another one still
/// *decodes* — the data is not lost — but cannot become a link yet.
#[test]
fn decodes_a_non_gtin_primary_key_but_cannot_build_a_link() {
    // AI 8003 (GRAI): a leading zero, a 13-digit key, then an optional
    // serial — variable length overall, so the value runs to the end.
    //
    // Note the input must not be written as `8003` + `0` + the key by
    // accident: `8030` is itself an assigned AI, and the prefix rule would
    // correctly read that instead. The AI here is the first four characters.
    let parsed = ElementString::parse("800309506000134352ASSET1").expect("decodes");
    assert_eq!(
        parsed.other,
        vec![("8003".to_owned(), "09506000134352ASSET1".to_owned())]
    );

    let err = parsed
        .to_digital_link("https://id.example.com")
        .expect_err("no GTIN present");
    assert!(matches!(err, DigitalLinkError::MissingGtin), "got {err:?}");
}
