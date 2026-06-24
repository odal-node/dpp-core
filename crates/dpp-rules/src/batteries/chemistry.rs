//! Battery chemistry restrictions — EU Regulation 2023/1542, Article 9.
//!
//! The mercury and cadmium prohibition thresholds are inherited from Batteries
//! Directive 2006/66/EC and carried forward unchanged into the Battery Regulation.
//! They have been in force since 2008 and are **not** subject to pending delegated
//! acts — they are hard limits.
//!
//! ## Mercury (all battery types)
//! No battery may contain more than 0.0005 % mercury by weight
//! (Art. 9(1) / former Directive Art. 4(1)).
//!
//! ## Cadmium (portable batteries only)
//! Portable batteries may not contain more than 0.002 % cadmium by weight
//! (Art. 9(1) / former Directive Art. 4(2)).
//! Exceptions for emergency/alarm systems, medical devices, and cordless power
//! tools are being phased out under Art. 88 of EU 2023/1542.
//!
//! ## Operating temperature range (cross-field)
//! If both `operatingTempMinC` and `operatingTempMaxC` are declared in the
//! battery DPP, the minimum must be strictly less than the maximum. JSON Schema
//! cannot express this comparison across two fields.

use alloc::{format, string::String};

// ── Prohibition thresholds ────────────────────────────────────────────────────

/// Maximum mercury content (% by weight) for any battery type.
/// Source: EU 2023/1542 Art. 9 / Batteries Directive 2006/66/EC Art. 4(1).
pub const MERCURY_MAX_CONTENT_PCT: f64 = 0.0005;

/// Maximum cadmium content (% by weight) for **portable** batteries.
/// Source: EU 2023/1542 Art. 9 / Batteries Directive 2006/66/EC Art. 4(2).
/// Industrial and EV batteries have no cadmium weight-percentage limit.
pub const CADMIUM_PORTABLE_MAX_CONTENT_PCT: f64 = 0.002;

/// Whether a declared mercury content percentage violates the EU prohibition.
/// Returns `true` (prohibited) when `content_pct > 0.0005`.
#[must_use]
pub fn mercury_content_prohibited(content_pct: f64) -> bool {
    content_pct > MERCURY_MAX_CONTENT_PCT
}

/// Whether a declared cadmium content percentage violates the EU prohibition
/// for **portable** batteries. Returns `true` (prohibited) when `content_pct > 0.002`.
///
/// Do not call this for industrial or EV batteries — the prohibition does not apply.
#[must_use]
pub fn cadmium_content_prohibited_for_portable(content_pct: f64) -> bool {
    content_pct > CADMIUM_PORTABLE_MAX_CONTENT_PCT
}

// ── Cross-field: operating temperature range ──────────────────────────────────

/// Validate that the declared operating temperature range is physically coherent:
/// `operatingTempMinC` must be strictly less than `operatingTempMaxC`.
///
/// Both fields are optional in the battery schema; this rule fires only when both
/// are present. A single absent field is not an error here.
pub fn validate_operating_temp_range(min_c: Option<f64>, max_c: Option<f64>) -> Result<(), String> {
    if let (Some(min), Some(max)) = (min_c, max_c)
        && min >= max
    {
        return Err(format!(
            "operatingTempMinC ({min}°C) must be less than operatingTempMaxC ({max}°C)"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mercury_at_and_below_threshold_allowed() {
        assert!(!mercury_content_prohibited(0.0));
        assert!(!mercury_content_prohibited(0.0005)); // exactly at limit — allowed
    }

    #[test]
    fn mercury_above_threshold_prohibited() {
        assert!(mercury_content_prohibited(0.0006));
        assert!(mercury_content_prohibited(1.0));
    }

    #[test]
    fn cadmium_at_and_below_threshold_allowed() {
        assert!(!cadmium_content_prohibited_for_portable(0.0));
        assert!(!cadmium_content_prohibited_for_portable(0.002)); // exactly at limit — allowed
    }

    #[test]
    fn cadmium_above_threshold_prohibited() {
        assert!(cadmium_content_prohibited_for_portable(0.0021));
    }

    #[test]
    fn temp_range_valid_cases() {
        assert!(validate_operating_temp_range(Some(-20.0), Some(60.0)).is_ok());
        assert!(validate_operating_temp_range(None, Some(60.0)).is_ok()); // partial — not an error
        assert!(validate_operating_temp_range(None, None).is_ok());
    }

    #[test]
    fn temp_range_min_greater_than_max_rejected() {
        let err = validate_operating_temp_range(Some(60.0), Some(-20.0)).unwrap_err();
        assert!(err.contains("operatingTempMinC"), "unexpected: {err}");
    }

    #[test]
    fn temp_range_equal_values_rejected() {
        let err = validate_operating_temp_range(Some(25.0), Some(25.0)).unwrap_err();
        assert!(err.contains("less than"), "unexpected: {err}");
    }
}
