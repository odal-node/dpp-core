//! Reusable field validation for sector plugins.
//!
//! [`Validator`] is a fluent collector: each `require_*` / `optional_*` method
//! records a [`PluginFieldError`] when a check fails and is chainable, so a
//! plugin's `validate_input` reads as a declarative list of field constraints.
//! [`Validator::finish`] reports *all* failures at once rather than stopping at
//! the first — better for surfacing form errors to a manufacturer.
//!
//! The free functions [`num`] and [`str_of`] are convenience readers for
//! `calculate_metrics` bodies.

use dpp_plugin_traits::{PluginError, PluginFieldError, PluginInput};
use serde_json::Value;

/// A present, non-null value for `key`, or `None` if absent/null.
fn present<'a>(input: &'a Value, key: &str) -> Option<&'a Value> {
    match input.get(key) {
        Some(Value::Null) | None => None,
        other => other,
    }
}

/// Read a finite number field (ignores absent/non-numeric/NaN/inf).
#[must_use]
pub fn num(input: &PluginInput, key: &str) -> Option<f64> {
    input
        .get(key)
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite())
}

/// Read a string field.
#[must_use]
pub fn str_of<'a>(input: &'a PluginInput, key: &str) -> Option<&'a str> {
    input.get(key).and_then(Value::as_str)
}

/// Fluent per-field validator. See module docs.
pub struct Validator<'a> {
    input: &'a Value,
    errors: Vec<PluginFieldError>,
}

impl<'a> Validator<'a> {
    #[must_use]
    pub fn new(input: &'a PluginInput) -> Self {
        Self {
            input,
            errors: Vec::new(),
        }
    }

    fn push_opt(&mut self, key: &str, err: Option<(&str, String)>) {
        if let Some((code, message)) = err {
            self.errors.push(PluginFieldError {
                field: format!("/{key}"),
                code: code.to_owned(),
                message,
            });
        }
    }

    /// Require a present, non-empty string.
    pub fn require_str(&mut self, key: &str) -> &mut Self {
        let err = match present(self.input, key) {
            None => Some(("missing", format!("{key} is required"))),
            Some(v) => match v.as_str() {
                Some(s) if !s.trim().is_empty() => None,
                Some(_) => Some(("empty", format!("{key} must not be empty"))),
                None => Some(("type", format!("{key} must be a string"))),
            },
        };
        self.push_opt(key, err);
        self
    }

    /// Require a string field whose value is one of `allowed`.
    pub fn require_enum(&mut self, key: &str, allowed: &[&str]) -> &mut Self {
        let err = match present(self.input, key).and_then(Value::as_str) {
            None => Some(("missing", format!("{key} is required"))),
            Some(s) if allowed.contains(&s) => None,
            Some(_) => Some(("out_of_range", format!("{key} must be one of {allowed:?}"))),
        };
        self.push_opt(key, err);
        self
    }

    /// Require a 14-digit GS1 GTIN string with a valid check digit.
    pub fn require_gtin(&mut self, key: &str) -> &mut Self {
        let err = match present(self.input, key).and_then(Value::as_str) {
            None => Some(("missing", format!("{key} is required"))),
            Some(g) if g.len() == 14 && g.bytes().all(|b| b.is_ascii_digit()) => {
                if gs1_check_digit_valid(g) {
                    None
                } else {
                    Some(("checksum", format!("{key} has an invalid GS1 check digit")))
                }
            }
            Some(_) => Some(("format", format!("{key} must be 14 digits"))),
        };
        self.push_opt(key, err);
        self
    }

    /// Require a recognized ISO 3166-1 alpha-2 country code.
    pub fn require_country(&mut self, key: &str) -> &mut Self {
        let err = match present(self.input, key).and_then(Value::as_str) {
            None => Some(("missing", format!("{key} is required"))),
            Some(c) if c.len() == 2 && c.bytes().all(|b| b.is_ascii_uppercase()) => {
                if ISO_3166_1_A2.binary_search(&c).is_ok() {
                    None
                } else {
                    Some((
                        "invalid",
                        format!("{key} is not a recognized ISO 3166-1 alpha-2 code"),
                    ))
                }
            }
            Some(_) => Some((
                "format",
                format!("{key} must be a 2-letter uppercase country code"),
            )),
        };
        self.push_opt(key, err);
        self
    }

    /// Require a present, finite number greater than 0.
    pub fn require_positive(&mut self, key: &str) -> &mut Self {
        let err = match num(self.input, key) {
            None => Some((
                "missing",
                format!("{key} is required and must be a finite number"),
            )),
            Some(v) if v <= 0.0 => Some(("out_of_range", format!("{key} must be greater than 0"))),
            Some(_) => None,
        };
        self.push_opt(key, err);
        self
    }

    /// Require a present, finite number greater than or equal to 0.
    pub fn require_non_negative(&mut self, key: &str) -> &mut Self {
        let err = match num(self.input, key) {
            None => Some((
                "missing",
                format!("{key} is required and must be a finite number"),
            )),
            Some(v) if v < 0.0 => Some(("out_of_range", format!("{key} must be 0 or greater"))),
            Some(_) => None,
        };
        self.push_opt(key, err);
        self
    }

    /// Require a present, finite number in `[0, 100]`.
    pub fn require_pct(&mut self, key: &str) -> &mut Self {
        let err = match num(self.input, key) {
            None => Some((
                "missing",
                format!("{key} is required and must be a number in 0..=100"),
            )),
            Some(v) if !(0.0..=100.0).contains(&v) => {
                Some(("out_of_range", format!("{key} must be in 0..=100")))
            }
            Some(_) => None,
        };
        self.push_opt(key, err);
        self
    }

    /// Require a present non-negative integer that is at least 1.
    pub fn require_positive_int(&mut self, key: &str) -> &mut Self {
        let err = match present(self.input, key).and_then(Value::as_u64) {
            None => Some((
                "missing",
                format!("{key} is required and must be a non-negative integer"),
            )),
            Some(0) => Some(("out_of_range", format!("{key} must be at least 1"))),
            Some(_) => None,
        };
        self.push_opt(key, err);
        self
    }

    /// Require a present boolean.
    pub fn require_bool(&mut self, key: &str) -> &mut Self {
        let err = match present(self.input, key) {
            None => Some(("missing", format!("{key} is required"))),
            Some(v) if v.is_boolean() => None,
            Some(_) => Some(("type", format!("{key} must be a boolean"))),
        };
        self.push_opt(key, err);
        self
    }

    /// Require a present, non-empty array.
    pub fn require_non_empty_array(&mut self, key: &str) -> &mut Self {
        let err = match present(self.input, key).and_then(Value::as_array) {
            None => Some(("missing", format!("{key} is required and must be an array"))),
            Some(a) if a.is_empty() => Some(("empty", format!("{key} must not be empty"))),
            Some(_) => None,
        };
        self.push_opt(key, err);
        self
    }

    /// If present (and non-null), the value must be a finite number in `[0, 100]`.
    pub fn optional_pct(&mut self, key: &str) -> &mut Self {
        let err = match present(self.input, key) {
            None => None,
            Some(v) => match v.as_f64().filter(|n| n.is_finite()) {
                Some(n) if (0.0..=100.0).contains(&n) => None,
                _ => Some(("out_of_range", format!("{key} must be a number in 0..=100"))),
            },
        };
        self.push_opt(key, err);
        self
    }

    /// Finish validation, returning every collected error at once.
    pub fn finish(&mut self) -> Result<(), PluginError> {
        if self.errors.is_empty() {
            Ok(())
        } else {
            Err(PluginError::ValidationErrors(std::mem::take(
                &mut self.errors,
            )))
        }
    }
}

/// GS1 check-digit validation for GTIN-14.
///
/// Even-indexed positions (0, 2, 4, … 12) are weighted ×3; odd-indexed ×1.
/// The check digit at position 13 must equal `(10 − sum mod 10) mod 10`.
fn gs1_check_digit_valid(gtin: &str) -> bool {
    let bytes = gtin.as_bytes();
    debug_assert_eq!(bytes.len(), 14, "caller must check length == 14 first");
    let sum: u32 = bytes[..13]
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            let d = (b - b'0') as u32;
            if i % 2 == 0 { d * 3 } else { d }
        })
        .sum();
    let expected = (10 - sum % 10) % 10;
    expected == (bytes[13] - b'0') as u32
}

/// All 249 ISO 3166-1 alpha-2 country codes, sorted for binary search.
const ISO_3166_1_A2: &[&str] = &[
    "AD", "AE", "AF", "AG", "AI", "AL", "AM", "AO", "AQ", "AR", "AS", "AT", "AU", "AW", "AX", "AZ",
    "BA", "BB", "BD", "BE", "BF", "BG", "BH", "BI", "BJ", "BL", "BM", "BN", "BO", "BQ", "BR", "BS",
    "BT", "BV", "BW", "BY", "BZ", "CA", "CC", "CD", "CF", "CG", "CH", "CI", "CK", "CL", "CM", "CN",
    "CO", "CR", "CU", "CV", "CW", "CX", "CY", "CZ", "DE", "DJ", "DK", "DM", "DO", "DZ", "EC", "EE",
    "EG", "EH", "ER", "ES", "ET", "FI", "FJ", "FK", "FM", "FO", "FR", "GA", "GB", "GD", "GE", "GF",
    "GG", "GH", "GI", "GL", "GM", "GN", "GP", "GQ", "GR", "GS", "GT", "GU", "GW", "GY", "HK", "HM",
    "HN", "HR", "HT", "HU", "ID", "IE", "IL", "IM", "IN", "IO", "IQ", "IR", "IS", "IT", "JE", "JM",
    "JO", "JP", "KE", "KG", "KH", "KI", "KM", "KN", "KP", "KR", "KW", "KY", "KZ", "LA", "LB", "LC",
    "LI", "LK", "LR", "LS", "LT", "LU", "LV", "LY", "MA", "MC", "MD", "ME", "MF", "MG", "MH", "MK",
    "ML", "MM", "MN", "MO", "MP", "MQ", "MR", "MS", "MT", "MU", "MV", "MW", "MX", "MY", "MZ", "NA",
    "NC", "NE", "NF", "NG", "NI", "NL", "NO", "NP", "NR", "NU", "NZ", "OM", "PA", "PE", "PF", "PG",
    "PH", "PK", "PL", "PM", "PN", "PR", "PS", "PT", "PW", "PY", "QA", "RE", "RO", "RS", "RU", "RW",
    "SA", "SB", "SC", "SD", "SE", "SG", "SH", "SI", "SJ", "SK", "SL", "SM", "SN", "SO", "SR", "SS",
    "ST", "SV", "SX", "SY", "SZ", "TC", "TD", "TF", "TG", "TH", "TJ", "TK", "TL", "TM", "TN", "TO",
    "TR", "TT", "TV", "TW", "TZ", "UA", "UG", "UM", "US", "UY", "UZ", "VA", "VC", "VE", "VG", "VI",
    "VN", "VU", "WF", "WS", "YE", "YT", "ZA", "ZM", "ZW",
];

#[cfg(test)]
mod tests {
    use super::*;
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
    fn readers_extract_values() {
        let input = json!({ "n": 3.5, "s": "hi", "bad": "x" });
        assert_eq!(num(&input, "n"), Some(3.5));
        assert_eq!(num(&input, "bad"), None);
        assert_eq!(str_of(&input, "s"), Some("hi"));
    }
}
