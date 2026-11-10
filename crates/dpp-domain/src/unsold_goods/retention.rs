//! [`KeepUntil`] — how long the documents behind a discard must be kept.

use chrono::{Months, NaiveDate};

use super::exemption::EnterpriseSize;

/// Where the disclosure covering a discard stands, which is what Impl. Reg. (EU)
/// 2026/2 Art. 4's five years run from.
///
/// Three states, because "no disclosure yet" is two different facts. An operator
/// whose disclosure is still to come has five years that have not started; one that
/// owes none has five years that will never start. Reading both as "not yet" holds
/// the second operator's documents for ever.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disclosure {
    /// No disclosure covers the discard, so Art. 4's period never starts. Either
    /// the operator is outside ESPR Arts. 24(1) and 25(1) (see
    /// [`EnterpriseSize::is_exempt_on`]), or the discard falls in a year before the
    /// first one the Regulation covers: Impl. Reg. (EU) 2026/2 Art. 1 applies from
    /// "the first full financial year after the date of application". This crate
    /// does not hold an operator's financial years, so the caller says so for the
    /// second case.
    NotOwed,
    /// One is owed and has not been made. Art. 1 allows twelve months after the end
    /// of the year, and an operator may be late.
    Pending,
    /// It was made on this date.
    MadeOn(NaiveDate),
}

impl Disclosure {
    /// The state for an operator of `size` whose disclosure of a discard made on
    /// `discarded_on` was made on `made_on`, if it has been.
    ///
    /// An exempt operator owes none, and a disclosure it made anyway starts nothing.
    /// Whether the year is one the Regulation covers is not known here: see
    /// [`Self::NotOwed`].
    #[must_use]
    pub fn of(size: EnterpriseSize, discarded_on: NaiveDate, made_on: Option<NaiveDate>) -> Self {
        if size.is_exempt_on(discarded_on) {
            return Self::NotOwed;
        }
        made_on.map_or(Self::Pending, Self::MadeOn)
    }
}

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
/// made: [`Disclosure::Pending`] gives [`Self::Open`]. Treating an open answer as a
/// date is how a late discloser's documents get deleted early. Where none is owed
/// ([`Disclosure::NotOwed`]) the second period never starts, and the five years
/// after the discard are all there is. For an operator that is itself exempt there
/// are no required documents (see `DiscardRecord::required_evidence`), so that
/// period binds nothing.
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
/// where the disclosure covering it stands.
///
/// A covering disclosure postdates the discard, so once it is made its five years
/// are the later; the discard's own five years decide where none is owed. A
/// disclosure dated before the discard is a mistake in the data, and the answer is
/// still never earlier than the discard's five years.
///
/// A date that is not on the calendar five years on — 29 February — falls on the
/// last day of the month, as `FinancialYear::disclosure_due_by` does. A date so far
/// out that adding five years overflows the calendar is kept to the end of it
/// rather than dropped: never earlier than the act requires.
#[must_use]
pub fn keep_until(discarded_on: NaiveDate, disclosure: Disclosure) -> KeepUntil {
    let after_discard = five_years_after(discarded_on);
    match disclosure {
        Disclosure::NotOwed => KeepUntil::On(after_discard),
        Disclosure::Pending => KeepUntil::Open {
            not_before: after_discard,
        },
        Disclosure::MadeOn(made_on) => KeepUntil::On(after_discard.max(five_years_after(made_on))),
    }
}

/// Five years after `date`, or the end of the calendar where that overflows.
fn five_years_after(date: NaiveDate) -> NaiveDate {
    date.checked_add_months(Months::new(60))
        .unwrap_or(NaiveDate::MAX)
}
