//! A dependency-free calendar date, used as an ordering key for regulatory
//! effective dates.
//!
//! `dpp-rules` is `no_std` with no dependencies, so it cannot use `chrono`.
//! Regulatory phase selection only ever needs to answer "is this date on or
//! after that date", which is a lexicographic comparison of (year, month, day)
//! — no calendar arithmetic, no time zones, no leap-year handling.
//!
//! Deliberately *not* a validated calendar date. Range checking belongs at the
//! ingestion boundary, where a bad value can be rejected with a field path and
//! a message; a rules crate that silently repaired one would hide the defect.

/// A calendar date, comparable by (year, month, day).
///
/// The derived `Ord` compares fields in declaration order, which for these
/// three fields is chronological order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct CalendarDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

impl CalendarDate {
    /// Construct a date from year, month and day.
    #[must_use]
    pub const fn new(year: i32, month: u8, day: u8) -> Self {
        Self { year, month, day }
    }

    /// Whether the month is 1–12 and the day 1–31.
    ///
    /// A coarse well-formedness check, not a calendar validation: it accepts
    /// 31 February. Use it to reject transposed or garbage input at an
    /// ingestion boundary, not to prove a date exists.
    #[must_use]
    pub const fn is_plausible(&self) -> bool {
        self.month >= 1 && self.month <= 12 && self.day >= 1 && self.day <= 31
    }

    /// Parse a strict `YYYY-MM-DD` date.
    ///
    /// Returns `None` for anything else — wrong length, wrong separators,
    /// non-digits, or components failing [`Self::is_plausible`]. Deliberately
    /// strict and deliberately not lenient about partial dates: a compliance
    /// determination keyed on a date must never run against one the declarer
    /// did not actually state.
    #[must_use]
    pub fn parse_iso(s: &str) -> Option<Self> {
        let b = s.as_bytes();
        if b.len() != 10 || b[4] != b'-' || b[7] != b'-' {
            return None;
        }
        let field = |from: usize, to: usize| -> Option<u32> {
            let mut acc: u32 = 0;
            for &c in &b[from..to] {
                if !c.is_ascii_digit() {
                    return None;
                }
                acc = acc * 10 + u32::from(c - b'0');
            }
            Some(acc)
        };
        let date = Self::new(
            i32::try_from(field(0, 4)?).ok()?,
            u8::try_from(field(5, 7)?).ok()?,
            u8::try_from(field(8, 10)?).ok()?,
        );
        date.is_plausible().then_some(date)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
