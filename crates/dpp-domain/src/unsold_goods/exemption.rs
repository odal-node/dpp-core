//! [`EnterpriseSize`] — who ESPR Arts. 24 and 25 reach.
//!
//! ✅ COMPLIANCE-PIN: Regulation (EU) 2024/1781 Art. 2 (the closing paragraph applying
//! the SME definitions of Recommendation 2003/361/EC) and Arts. 24(1), 24(2), 25(1),
//! 25(2) and 25(5) (consolidated 28.6.2024); Impl. Reg. (EU) 2026/2 recital 1
//! (OJ L, 10.2.2026). Del. Reg. (EU) 2026/296 (OJ L, 22.4.2026) was read for the
//! same and says nothing about micro, small or medium-sized enterprises. Read against
//! the Official Journal texts.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// The day the disclosure and the destruction ban start to reach medium-sized
/// enterprises: 19 July 2030.
pub const MEDIUM_ENTERPRISES_FROM: NaiveDate = match NaiveDate::from_ymd_opt(2030, 7, 19) {
    Some(date) => date,
    None => panic!("19 July 2030 is a date"),
};

/// The size class of an economic operator, as ESPR Arts. 24(1) and 25(1) sort them.
///
/// Both paragraphs end the same way: *"This paragraph shall not apply to micro and
/// small enterprises"*, and *"shall apply to medium-sized enterprises from 19 July
/// 2030"*. Art. 24(1) is the yearly disclosure and Art. 25(1) the ban on destroying
/// the products of Annex VII, so an operator is either inside both or outside both.
/// Impl. Reg. (EU) 2026/2 recital 1 reads Art. 24(1) the same way: large enterprises
/// and, from that date, medium-sized ones.
///
/// The classes are those of Commission Recommendation 2003/361/EC, which ESPR Art. 2
/// applies. Which one an operator belongs to depends on its headcount and its
/// turnover or balance sheet, which this crate does not hold, so the host says.
///
/// # Outside both, nothing is owed
///
/// An operator outside Art. 24(1) discloses nothing, so Art. 24(2) has no disclosure
/// for its documentation to demonstrate; one outside Art. 25(1) may destroy, so no
/// derogation needs documenting. Art. 25(2) still bars it from destroying products
/// supplied to it in order to circumvent the ban, but attaches no record-keeping to
/// that. Art. 25(5) lets a delegated act extend the ban or the disclosure to micro
/// and small enterprises that are shown to be used for circumvention; Del. Reg. (EU)
/// 2026/296 does not.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum EnterpriseSize {
    /// A microenterprise. Never reached.
    Micro,
    /// A small enterprise. Never reached.
    Small,
    /// A medium-sized enterprise. Reached from [`MEDIUM_ENTERPRISES_FROM`].
    Medium,
    /// Anything larger. Reached throughout.
    Large,
}

impl EnterpriseSize {
    /// Whether Arts. 24(1) and 25(1) leave an operator of this size alone on `date`.
    ///
    /// `date` is the day of the discard: it is the day the ban either bound the
    /// operator or did not.
    #[must_use]
    pub fn is_exempt_on(self, date: NaiveDate) -> bool {
        match self {
            Self::Micro | Self::Small => true,
            Self::Medium => date < MEDIUM_ENTERPRISES_FROM,
            Self::Large => false,
        }
    }
}
