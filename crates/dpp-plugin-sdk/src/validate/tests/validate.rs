//! Tests of the `validate` module.

use super::super::*;
use dpp_plugin_traits::PluginError;
use serde_json::json;

#[test]
fn collects_all_failures() {
    let input = json!({ "gtin": "12-34", "voltage": -1.0 });
    let err = Validator::new(&input)
        .require_gtin("gtin")
        .require_positive("voltage")
        .require_str("name")
        .finish()
        .unwrap_err();
    match err {
        PluginError::ValidationErrors(errs) => assert_eq!(errs.len(), 3),
        other => panic!("expected ValidationErrors, got {other:?}"),
    }
}

#[test]
fn valid_input_passes() {
    let input = json!({
        "gtin": "12345678901231",
        "country": "DE",
        "pct": 42.0,
        "count": 3,
        "flag": true,
        "items": [1]
    });
    assert!(
        Validator::new(&input)
            .require_gtin("gtin")
            .require_country("country")
            .require_pct("pct")
            .require_positive_int("count")
            .require_bool("flag")
            .require_non_empty_array("items")
            .finish()
            .is_ok()
    );
}

#[test]
fn enum_and_country_and_pct_bounds() {
    let input = json!({ "cls": "Z", "country": "de", "pct": 150.0 });
    let err = Validator::new(&input)
        .require_enum("cls", &["A", "B"])
        .require_country("country")
        .require_pct("pct")
        .finish()
        .unwrap_err();
    match err {
        PluginError::ValidationErrors(errs) => assert_eq!(errs.len(), 3),
        other => panic!("expected ValidationErrors, got {other:?}"),
    }
}

#[test]
fn gtin_invalid_check_digit_is_rejected() {
    // "12345678901234" — correct digits, correct length, but check digit should be 1, not 4.
    let input = json!({ "gtin": "12345678901234" });
    let err = Validator::new(&input)
        .require_gtin("gtin")
        .finish()
        .unwrap_err();
    match err {
        PluginError::ValidationErrors(errs) => {
            assert_eq!(errs.len(), 1);
            assert_eq!(errs[0].code, "checksum");
        }
        other => panic!("expected ValidationErrors, got {other:?}"),
    }
}

#[test]
fn gtin_valid_check_digit_passes() {
    // "12345678901231" — check digit = 1, matches GS1 calculation.
    let input = json!({ "gtin": "12345678901231" });
    assert!(Validator::new(&input).require_gtin("gtin").finish().is_ok());
}

#[test]
fn country_not_in_iso_list_is_rejected() {
    // "XX" has the right format (2 uppercase letters) but is not an assigned code.
    let input = json!({ "country": "XX" });
    let err = Validator::new(&input)
        .require_country("country")
        .finish()
        .unwrap_err();
    match err {
        PluginError::ValidationErrors(errs) => {
            assert_eq!(errs.len(), 1);
            assert_eq!(errs[0].code, "invalid");
        }
        other => panic!("expected ValidationErrors, got {other:?}"),
    }
}

#[test]
fn country_valid_iso_code_passes() {
    for code in ["DE", "NO", "FR", "US", "JP"] {
        let input = json!({ "country": code });
        assert!(
            Validator::new(&input)
                .require_country("country")
                .finish()
                .is_ok(),
            "{code} should be a valid ISO 3166-1 alpha-2 code"
        );
    }
}

#[test]
fn optional_pct_absent_is_ok_present_out_of_range_fails() {
    let ok = json!({});
    assert!(Validator::new(&ok).optional_pct("x").finish().is_ok());
    let bad = json!({ "x": 101.0 });
    assert!(Validator::new(&bad).optional_pct("x").finish().is_err());
}

#[test]
fn optional_range_absent_ok_present_bounded() {
    // Absent → ok (the field's absence is meaningful).
    assert!(
        Validator::new(&json!({}))
            .optional_range("s", 0.0, 10.0)
            .finish()
            .is_ok()
    );
    // In range → ok.
    assert!(
        Validator::new(&json!({ "s": 6.0 }))
            .optional_range("s", 0.0, 10.0)
            .finish()
            .is_ok()
    );
    // Out of range → err (the fail-open case the plugins guard against).
    for bad in [json!({ "s": 999999.0 }), json!({ "s": -1.0 })] {
        assert!(
            Validator::new(&bad)
                .optional_range("s", 0.0, 10.0)
                .finish()
                .is_err()
        );
    }
}

#[test]
fn optional_non_negative_absent_ok_negative_fails() {
    assert!(
        Validator::new(&json!({}))
            .optional_non_negative("c")
            .finish()
            .is_ok()
    );
    assert!(
        Validator::new(&json!({ "c": 0.0 }))
            .optional_non_negative("c")
            .finish()
            .is_ok()
    );
    assert!(
        Validator::new(&json!({ "c": -999.0 }))
            .optional_non_negative("c")
            .finish()
            .is_err()
    );
}

#[test]
fn readers_extract_values() {
    let input = json!({ "n": 3.5, "s": "hi", "bad": "x" });
    assert_eq!(num(&input, "n"), Some(3.5));
    assert_eq!(num(&input, "bad"), None);
    assert_eq!(str_of(&input, "s"), Some("hi"));
}
