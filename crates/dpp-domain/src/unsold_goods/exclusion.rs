//! [`ExcludedRecord`] — a record the aggregation could not count, and why.

use serde::Serialize;

/// Why a record is left out of a year's disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum ExclusionReason {
    /// Discarded outside the financial year being disclosed. It belongs to another
    /// year's disclosure; it is reported rather than summed into this one.
    OutsideFinancialYear,
    /// The record names no CN code, so Art. 3 has nothing to file it under.
    NoCnCode,
    /// A CN code on the record is not 6, 8 or 10 digits.
    UnreadableCnCode,
    /// The record has no weight, so its share of a line's treatment split cannot
    /// be worked out. Annex I note (g) lets the weight be estimated from an
    /// accurate count; a record carries that estimate, not a zero.
    NoWeight,
}

/// A record the aggregation left out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExcludedRecord {
    /// Position of the record in the slice that was aggregated.
    pub index: usize,
    /// Why it was left out.
    pub why: ExclusionReason,
}
