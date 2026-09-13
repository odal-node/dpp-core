//! [`CurrencyCheck`] — a currency state, and the date it was established on.

use serde::{Deserialize, Serialize};

use super::state::CurrencyState;

/// What a status check found, and when it was made.
///
/// # This records the check, not the act
///
/// The distinction is the whole point of the type. Nothing here claims an act
/// *is* in force; it claims that on [`checked_on`](Self::checked_on) someone
/// looked and found it so. An act can be repealed the day after a check and
/// this record stays truthful — it is a dated observation, and a reader can see
/// how old it is.
///
/// A currency field on its own would decay silently: every entry would read as
/// current forever, and a check made two years ago would be indistinguishable
/// from one made this morning. The date is the half that keeps working.
///
/// # A struct rather than two fields on the act
///
/// The same reason [`ObligationDate`](crate::instrument::ObligationDate) is a
/// struct: a state cannot then exist without its date, and there is no second
/// optional field to fall out of step with the first. An act either carries a
/// dated check or carries nothing.
///
/// # Serialised flat
///
/// [`state`](Self::state) is flattened, so the wire form is one object rather
/// than a state nested inside a wrapper:
/// `{"state":"consolidated","asOf":"02023R1542-20260813","checkedOn":"2026-09-11"}`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CurrencyCheck {
    /// What the check found.
    #[serde(flatten)]
    pub state: CurrencyState,
    /// ISO-8601 date the check was made.
    ///
    /// Fixed format because it is compared as a string — the same convention
    /// [`ObligationDate::date`](crate::instrument::ObligationDate::date) uses,
    /// and `YYYY-MM-DD` orders correctly under lexicographic comparison.
    pub checked_on: String,
}

impl CurrencyCheck {
    /// Whether this check is older than `date`.
    ///
    /// The query behind any staleness report. Deliberately a query and not a
    /// gate: see
    /// [`InstrumentCatalog::currency_checked_before`](crate::instrument::InstrumentCatalog::currency_checked_before).
    #[must_use]
    pub fn is_older_than(&self, date: &str) -> bool {
        self.checked_on.as_str() < date
    }
}
