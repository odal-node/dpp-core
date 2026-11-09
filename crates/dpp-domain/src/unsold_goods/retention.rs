//! [`KeepUntil`] — how long the documents behind a discard must be kept.

use chrono::{Months, NaiveDate};

/// Until when a record's documents must be kept.
///
/// Two provisions set a period, and a record's documents are held for the **later**
/// of them:
///
/// - **Del. Reg. (EU) 2026/296 Art. 3**: "for a period of five years after an
///   unsold consumer product subject to a derogation … has been destroyed".
/// - **Impl. Reg. (EU) 2026/2 Art. 4**: "for five years after the disclosure of
///   information on such products".
///
/// The second cannot be computed until the disclosure covering the discard has been
/// made — Art. 1 allows twelve months after the end of the financial year, and an
/// operator may be late. Until then the answer is [`Self::Open`]: not before the
/// first date, and for five years after the disclosure once there is one. Treating
/// an open answer as a date is how a late discloser's documents get deleted early.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepUntil {
    /// The last day the documents must still be held; they may go the day after.
    On(NaiveDate),
    /// The covering disclosure has not been made, so the five years after it have
    /// not started. The documents must be held at least until `not_before`, and
    /// longer.
    Open {
        /// Five years after the discard, the earliest the duty can end.
        not_before: NaiveDate,
    },
}

impl KeepUntil {
    /// Whether the documents may be deleted on `today`. Never while the answer is
    /// [`Self::Open`].
    #[must_use]
    pub fn permits_deletion_on(self, today: NaiveDate) -> bool {
        match self {
            Self::On(last_day) => today > last_day,
            Self::Open { .. } => false,
        }
    }
}

/// How long to keep the documents behind a discard made on `discarded_on`, given
/// the date the disclosure covering it was made, if it has been.
///
/// A date that is not on the calendar five years on — 29 February — falls on the
/// last day of the month, as `FinancialYear::disclosure_due_by` does. A date so far
/// out that adding five years overflows the calendar is kept to the end of it
/// rather than dropped: never earlier than the act requires.
#[must_use]
pub fn keep_until(discarded_on: NaiveDate, disclosed_on: Option<NaiveDate>) -> KeepUntil {
    let after_discard = five_years_after(discarded_on);
    match disclosed_on {
        Some(disclosed_on) => KeepUntil::On(after_discard.max(five_years_after(disclosed_on))),
        None => KeepUntil::Open {
            not_before: after_discard,
        },
    }
}

fn five_years_after(date: NaiveDate) -> NaiveDate {
    date.checked_add_months(Months::new(60))
        .unwrap_or(NaiveDate::MAX)
}
