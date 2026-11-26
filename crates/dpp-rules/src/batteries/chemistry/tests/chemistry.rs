//! Tests of the `chemistry` module.

use super::super::*;

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
