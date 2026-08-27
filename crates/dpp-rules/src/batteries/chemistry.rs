//! Battery substance restrictions — Regulation (EU) 2023/1542 **Art. 6** and
//! **Annex I**.
//!
//! Art. 6(1) is the operative prohibition: batteries shall not contain
//! substances for which Annex I sets a restriction, other than under the
//! conditions Annex I states. The thresholds themselves are in Annex I, which
//! has exactly three entries.
//!
//! This module cited **Art. 9** for all of it until 2026-08-27. Art. 9 is
//! *Performance and durability requirements for portable batteries of general
//! use* and says nothing about substances. The thresholds were right and the
//! citation was not, which is the failure mode that survives longest: checking
//! it requires opening the act.
//!
//! ## Annex I entry 1 — mercury, all battery types
//!
//! *"Batteries, whether or not incorporated into appliances, light means of
//! transport or other vehicles, shall not contain more than 0,0005 % of mercury
//! (expressed as mercury metal) by weight."* No date gate — in force now.
//!
//! ## Annex I entry 2 — cadmium, portable batteries only
//!
//! *"Portable batteries … shall not contain more than 0,002 % of cadmium
//! (expressed as cadmium metal) by weight."* No date gate. Industrial and EV
//! batteries carry no cadmium weight-percentage limit.
//!
//! ## Annex I entry 3 — lead, portable batteries, dated
//!
//! *"From 18 August 2024, portable batteries, whether or not incorporated into
//! appliances, shall not contain more than 0,01 % of lead (expressed as lead
//! metal) by weight"*, and *"the restriction … shall not apply to portable
//! zinc-air button cells until 18 August 2028"*.
//!
//! Alone among the three it has both a start date and a carve-out, so it is the
//! only one whose answer depends on when the battery was placed on the market
//! and what kind of cell it is. It was also the one missing entirely from this
//! module while its two siblings were implemented.
//!
//! ## Operating temperature range (cross-field)
//! If both `operatingTempMinC` and `operatingTempMaxC` are declared in the
//! battery DPP, the minimum must be strictly less than the maximum. JSON Schema
//! cannot express this comparison across two fields.

use alloc::{format, string::String};

use crate::common::date::CalendarDate;

// ── Annex I thresholds ────────────────────────────────────────────────────────

/// Maximum mercury content (% by weight) for any battery type.
///
/// Annex I entry 1, under Art. 6(1).
pub const MERCURY_MAX_CONTENT_PCT: f64 = 0.0005;

/// Maximum cadmium content (% by weight) for **portable** batteries.
///
/// Annex I entry 2, under Art. 6(1). Industrial and EV batteries have no
/// cadmium weight-percentage limit.
pub const CADMIUM_PORTABLE_MAX_CONTENT_PCT: f64 = 0.002;

/// Maximum lead content (% by weight) for **portable** batteries.
///
/// Annex I entry 3, under Art. 6(1).
pub const LEAD_PORTABLE_MAX_CONTENT_PCT: f64 = 0.01;

/// First day the Annex I lead restriction applies — 18 August 2024.
pub const LEAD_PORTABLE_RESTRICTION_FROM: CalendarDate = CalendarDate::new(2024, 8, 18);

/// The day the portable zinc-air button cell carve-out ends — 18 August 2028.
///
/// Annex I entry 3 point 2 disapplies the lead restriction to those cells
/// *"until"* this date, so it binds them **from** it.
pub const LEAD_ZINC_AIR_BUTTON_CELL_BOUND_FROM: CalendarDate = CalendarDate::new(2028, 8, 18);

/// Whether a declared mercury content percentage violates the Annex I entry 1
/// prohibition. Returns `true` (prohibited) when `content_pct > 0.0005`.
#[must_use]
pub fn mercury_content_prohibited(content_pct: f64) -> bool {
    content_pct > MERCURY_MAX_CONTENT_PCT
}

/// Whether a declared cadmium content percentage violates the Annex I entry 2
/// prohibition for **portable** batteries. Returns `true` (prohibited) when
/// `content_pct > 0.002`.
///
/// Do not call this for industrial or EV batteries — the prohibition does not apply.
#[must_use]
pub fn cadmium_content_prohibited_for_portable(content_pct: f64) -> bool {
    content_pct > CADMIUM_PORTABLE_MAX_CONTENT_PCT
}

/// Whether a declared lead content percentage violates the Annex I entry 3
/// prohibition for **portable** batteries.
///
/// Unlike mercury and cadmium this one is dated, so it needs the date the
/// battery was placed on the EU market. A battery placed before 18 August 2024
/// is not in breach however much lead it declares, and reporting one as
/// prohibited would misstate an operator's position exactly as
/// [`super::recycled_content::art8_phase_for`] refuses to for Art. 8.
///
/// `zinc_air_button_cell` moves the binding date to 18 August 2028 under Annex I
/// entry 3 point 2. It is a separate argument rather than a battery type because
/// the carve-out is about the cell chemistry and form, which the type enum does
/// not record.
///
/// Do not call this for industrial or EV batteries — entry 3 names portable
/// batteries only.
#[must_use]
pub fn lead_content_prohibited_for_portable(
    content_pct: f64,
    placed_on_market: CalendarDate,
    zinc_air_button_cell: bool,
) -> bool {
    let binds_from = if zinc_air_button_cell {
        LEAD_ZINC_AIR_BUTTON_CELL_BOUND_FROM
    } else {
        LEAD_PORTABLE_RESTRICTION_FROM
    };
    placed_on_market >= binds_from && content_pct > LEAD_PORTABLE_MAX_CONTENT_PCT
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

    // ── Annex I entry 3 — lead in portable batteries ─────────────────────────
    //
    // Added 2026-08-27. The entry was absent while its two siblings from the
    // same annex were implemented, so every test below fails against the
    // previous version by not compiling at all.

    /// Over the limit, and placed after the restriction began.
    #[test]
    fn lead_above_the_limit_is_prohibited_once_the_restriction_binds() {
        assert!(lead_content_prohibited_for_portable(
            0.02,
            CalendarDate::new(2025, 1, 1),
            false
        ));
    }

    /// The threshold is *more than* 0,01 %, so exactly at it is compliant.
    #[test]
    fn lead_exactly_at_the_limit_is_permitted() {
        assert!(!lead_content_prohibited_for_portable(
            LEAD_PORTABLE_MAX_CONTENT_PCT,
            CalendarDate::new(2025, 1, 1),
            false
        ));
    }

    /// **The date gate.** A battery placed before 18 Aug 2024 is not in breach
    /// however much lead it declares — the restriction had not begun. Reporting
    /// it as prohibited would be a retroactive finding, which is the same error
    /// `art8_phase_for` exists to avoid for Art. 8.
    #[test]
    fn lead_over_the_limit_is_not_prohibited_before_the_restriction_binds() {
        assert!(!lead_content_prohibited_for_portable(
            5.0,
            CalendarDate::new(2024, 8, 17),
            false
        ));
        assert!(
            lead_content_prohibited_for_portable(5.0, CalendarDate::new(2024, 8, 18), false),
            "18 Aug 2024 is the first day it binds"
        );
    }

    /// Annex I entry 3 point 2 exempts portable zinc-air button cells "until"
    /// 18 Aug 2028, so it binds them from that day and not before.
    #[test]
    fn zinc_air_button_cells_are_exempt_until_2028() {
        let over = 0.5;
        assert!(
            !lead_content_prohibited_for_portable(over, CalendarDate::new(2028, 8, 17), true),
            "still inside the carve-out"
        );
        assert!(
            lead_content_prohibited_for_portable(over, CalendarDate::new(2028, 8, 18), true),
            "the carve-out ends and the restriction binds"
        );
        assert!(
            lead_content_prohibited_for_portable(over, CalendarDate::new(2025, 1, 1), false),
            "a non-zinc-air portable cell had no carve-out to begin with"
        );
    }
}
