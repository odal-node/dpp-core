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
mod tests;
