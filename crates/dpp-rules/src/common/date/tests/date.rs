//! Tests of the `date` module.

use super::super::*;

#[test]
fn orders_chronologically_across_field_boundaries() {
    let earlier = CalendarDate::new(2031, 8, 17);
    let boundary = CalendarDate::new(2031, 8, 18);
    assert!(earlier < boundary);
    // Year dominates month, month dominates day.
    assert!(CalendarDate::new(2030, 12, 31) < CalendarDate::new(2031, 1, 1));
    assert!(CalendarDate::new(2031, 7, 31) < CalendarDate::new(2031, 8, 1));
}

#[test]
fn equal_dates_compare_equal() {
    assert_eq!(
        CalendarDate::new(2036, 8, 18),
        CalendarDate::new(2036, 8, 18)
    );
}

#[test]
fn parses_a_well_formed_iso_date() {
    assert_eq!(
        CalendarDate::parse_iso("2031-08-18"),
        Some(CalendarDate::new(2031, 8, 18))
    );
    assert_eq!(
        CalendarDate::parse_iso("0001-01-01"),
        Some(CalendarDate::new(1, 1, 1))
    );
}

#[test]
fn rejects_anything_that_is_not_strict_iso() {
    for bad in [
        "",
        "2031-8-18",   // unpadded month
        "2031/08/18",  // wrong separator
        "31-08-2031",  // wrong order
        "2031-08-18T", // trailing content
        "2031-08-1",   // too short
        "2031-08-188", // too long
        "20x1-08-18",  // non-digit
        "2031-13-01",  // implausible month
        "2031-00-01",
        "2031-08-00",
        "2031-08-32",
    ] {
        assert_eq!(CalendarDate::parse_iso(bad), None, "should reject {bad:?}");
    }
}

#[test]
fn plausibility_rejects_out_of_range_components() {
    assert!(CalendarDate::new(2031, 8, 18).is_plausible());
    assert!(!CalendarDate::new(2031, 13, 1).is_plausible());
    assert!(!CalendarDate::new(2031, 0, 1).is_plausible());
    assert!(!CalendarDate::new(2031, 8, 0).is_plausible());
    assert!(!CalendarDate::new(2031, 8, 32).is_plausible());
    // Coarse by design: month length is not checked.
    assert!(CalendarDate::new(2031, 2, 31).is_plausible());
}
