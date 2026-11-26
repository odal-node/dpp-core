//! Sample discard records and helpers the unsold-goods tests share.

use chrono::NaiveDate;

use crate::test_support::sample_discard_record;
use crate::unsold_goods::{CnCategory, DiscardedProductLine, FinancialYear};
use crate::unsold_goods::{DiscardRecord, LineDescriptions, aggregate};

/// A calendar date, for readability.
pub(super) fn date(y: i32, m: u32, d: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(y, m, d).expect("valid date")
}

/// The financial year the tests aggregate: the calendar year 2027.
pub(super) fn year() -> FinancialYear {
    FinancialYear {
        start: date(2027, 1, 1),
        end: date(2027, 12, 31),
    }
}

/// The sample record, which falls in `year()`.
pub(super) fn record() -> DiscardRecord {
    sample_discard_record()
}

/// The lines a set of records aggregates to, with no wording supplied by the discloser.
pub(super) fn lines(records: &[DiscardRecord]) -> Vec<DiscardedProductLine> {
    aggregate(records, &year(), &LineDescriptions::new()).lines
}

/// A CN category from its digits.
pub(super) fn category(code: &str) -> CnCategory {
    CnCategory::parse(code).expect("valid category")
}
