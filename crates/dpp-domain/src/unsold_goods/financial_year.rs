//! [`FinancialYear`] — the period a disclosure covers.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// The financial year a disclosure reports on, by its start and end dates.
///
/// # Why a financial year and not a calendar period
///
/// **Commission Implementing Regulation (EU) 2026/2, Art. 1:** the Regulation
/// "shall apply to products discarded in **each financial year** as from the
/// first full financial year after the date of application of this Regulation.
/// Economic operators shall disclose that information **within 12 months after
/// the end of that financial year**."
///
/// A financial year is the undertaking's own, so it is not derivable from a year
/// number and does not necessarily start in January. Annex I asks for both
/// endpoints as `dd/mm/yyyy` for exactly that reason, and the previous model's
/// free-text `"2026-Q2"` could not express it — a quarter is not a period this
/// disclosure is ever made for.
///
/// Ordering is not enforced by the type. A start after its end is a malformed
/// disclosure that should be *reported*, not made unreadable; the check lives
/// with the other cross-field rules in `dpp-rules`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FinancialYear {
    /// First day of the financial year.
    pub start: NaiveDate,
    /// Last day of the financial year.
    pub end: NaiveDate,
}

impl FinancialYear {
    /// The date by which this year's disclosure is due — 12 months after the
    /// end of the financial year, per Art. 1.
    ///
    /// `None` only if the addition overflows the calendar, which no real
    /// financial year does.
    #[must_use]
    pub fn disclosure_due_by(&self) -> Option<NaiveDate> {
        self.end.checked_add_months(chrono::Months::new(12))
    }
}
