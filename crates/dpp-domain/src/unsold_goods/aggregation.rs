//! [`Aggregation`] — a financial year's discards, as the lines Annex I asks for.

use std::collections::{BTreeMap, BTreeSet};

use dpp_rules::unsold_goods::disclosure::category_of_cn_code;

use super::apportion::whole_percentages;
use super::exclusion::{ExcludedRecord, ExclusionReason};
use super::operation::WasteTreatmentOperation;
use super::record::DiscardRecord;
use crate::unsold_goods::{
    CnCategory, DiscardReason, DiscardedProductLine, DiscardedQuantity, FinancialYear,
    WasteTreatmentSplit,
};

/// The lines a financial year's records make, and the records left out.
///
/// `#[must_use]` because `excluded` is part of the answer: a disclosure built from
/// `lines` alone under-reports whatever is in it.
#[derive(Debug, Clone, PartialEq)]
#[must_use]
pub struct Aggregation {
    /// The body of the Annex I table, in a fixed order.
    pub lines: Vec<DiscardedProductLine>,
    /// Records that could not be counted, each with the reason.
    pub excluded: Vec<ExcludedRecord>,
}

/// What a line is made from: records sharing it are one row of the table.
///
/// Annex I note (h) makes the reason part of it, and the packaging column is a
/// single answer per row, so records that differ on either cannot share one. The
/// explanation is part of it because where no derogation applies it is the only
/// statement of the reason. The derived order is the order lines come out in:
/// category, then derogation by Art. 2 point with those that cite none last, then the
/// explanation, then packaging excluded before included.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct LineKey {
    categories: Vec<CnCategory>,
    /// Position of the derogation in [`DiscardReason::ALL`], which is Art. 2's
    /// order; one past the end where none is cited, so those lines come last.
    reason_rank: usize,
    detail: Option<String>,
    packaging_included: bool,
}

impl LineKey {
    fn rank_of(reason: Option<DiscardReason>) -> usize {
        reason
            .and_then(|reason| DiscardReason::ALL.iter().position(|r| *r == reason))
            .unwrap_or(DiscardReason::ALL.len())
    }

    fn reason(&self) -> Option<DiscardReason> {
        DiscardReason::ALL.get(self.reason_rank).copied()
    }
}

/// What the records of one line add up to so far.
#[derive(Default)]
struct Totals {
    units: u64,
    units_estimated: bool,
    grams: u64,
    weight_estimated: bool,
    descriptions: BTreeSet<String>,
    /// Grams per operation, in the column order of [`WasteTreatmentSplit`].
    grams_by_operation: [u64; 5],
}

fn operation_column(operation: WasteTreatmentOperation) -> usize {
    match operation {
        WasteTreatmentOperation::PreparingForReuse => 0,
        WasteTreatmentOperation::Recycling => 1,
        WasteTreatmentOperation::OtherRecovery => 2,
        WasteTreatmentOperation::Disposal => 3,
        WasteTreatmentOperation::Unknown => 4,
    }
}

/// The categories Art. 3 files a record's CN codes under, sorted and without
/// repeats; `Err` for a record that names none, or one that cannot be read.
fn categories_of(record: &DiscardRecord) -> Result<Vec<CnCategory>, ExclusionReason> {
    if record.cn_codes.is_empty() {
        return Err(ExclusionReason::NoCnCode);
    }
    let mut categories = BTreeSet::new();
    for code in &record.cn_codes {
        let category = category_of_cn_code(code.as_str())
            .and_then(|category| CnCategory::parse(category).ok())
            .ok_or(ExclusionReason::UnreadableCnCode)?;
        categories.insert(category);
    }
    Ok(categories.into_iter().collect())
}

/// Aggregate discard records into the lines of one financial year's disclosure.
///
/// - A record counts toward the year its `discarded_on` falls in, both ends
///   included. One from another year is returned in `excluded`, as is one that
///   names no readable CN code or has no weight: none can be counted, and none is
///   dropped silently.
/// - Each CN code is filed under the category Art. 3 gives it (two digits, four for
///   Annex II). Items sold together (note (f)) are filed under the set of
///   categories they name, which is its own line.
/// - Units are summed; weights are summed in grams and rounded to whole
///   kilogrammes once, half up. A total is marked an estimate if any figure in it is.
/// - The treatment split is each operation's share of the line's **weight** (note
///   (i)), in whole percentages totalling exactly 100.
/// - A line's description is the distinct descriptions of its records, sorted and
///   joined with `"; "` — "a more detailed description" in the sense of note (e).
///
/// The disclosing entity and the two measures rows are not here: they are the
/// discloser's to supply.
pub fn aggregate(records: &[DiscardRecord], year: &FinancialYear) -> Aggregation {
    let mut lines: BTreeMap<LineKey, Totals> = BTreeMap::new();
    let mut excluded = Vec::new();

    for (index, record) in records.iter().enumerate() {
        let admitted = if record.discarded_on < year.start || record.discarded_on > year.end {
            Err(ExclusionReason::OutsideFinancialYear)
        } else {
            categories_of(record).and_then(|categories| {
                if record.weight.grams == 0 {
                    Err(ExclusionReason::NoWeight)
                } else {
                    Ok(categories)
                }
            })
        };
        let categories = match admitted {
            Ok(categories) => categories,
            Err(why) => {
                excluded.push(ExcludedRecord { index, why });
                continue;
            }
        };

        let key = LineKey {
            categories,
            reason_rank: LineKey::rank_of(record.reason),
            detail: record.reason_detail.clone(),
            packaging_included: record.packaging_included,
        };
        let totals = lines.entry(key).or_default();
        totals.units = totals.units.saturating_add(record.units.value);
        totals.units_estimated |= record.units.estimated;
        totals.grams = totals.grams.saturating_add(record.weight.grams);
        totals.weight_estimated |= record.weight.estimated;
        let description = record.description.trim();
        if !description.is_empty() {
            totals.descriptions.insert(description.to_owned());
        }
        let column = operation_column(record.treatment);
        totals.grams_by_operation[column] =
            totals.grams_by_operation[column].saturating_add(record.weight.grams);
    }

    let lines = lines
        .into_iter()
        .map(|(key, totals)| line_of(key, totals))
        .collect();

    Aggregation { lines, excluded }
}

fn line_of(key: LineKey, totals: Totals) -> DiscardedProductLine {
    let [
        preparing_for_reuse_pct,
        recycling_pct,
        other_recovery_pct,
        disposal_pct,
        unknown_pct,
    ] = whole_percentages(totals.grams_by_operation);
    let reason = key.reason();

    DiscardedProductLine {
        cn_categories: key.categories,
        description: totals
            .descriptions
            .into_iter()
            .collect::<Vec<_>>()
            .join("; "),
        units_discarded: DiscardedQuantity {
            value: totals.units,
            estimated: totals.units_estimated,
        },
        weight_kg: DiscardedQuantity {
            value: totals.grams.saturating_add(500) / 1000,
            estimated: totals.weight_estimated,
        },
        packaging_included: key.packaging_included,
        reason,
        reason_detail: key.detail,
        treatment: WasteTreatmentSplit {
            preparing_for_reuse_pct,
            recycling_pct,
            other_recovery_pct,
            disposal_pct,
            unknown_pct,
        },
    }
}
