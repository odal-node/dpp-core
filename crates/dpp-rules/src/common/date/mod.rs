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

mod calendar_date;
#[cfg(test)]
mod tests;

pub use calendar_date::*;
