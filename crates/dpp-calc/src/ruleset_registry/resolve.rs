//! `all_rulesets()` plus the per-methodology date-based `resolve_*` lookups.

use chrono::NaiveDate;

use crate::co2e::CradleToGateRuleset;
use crate::repairability::thresholds::{
    DisplaysRuleset, LaptopRuleset, RepairabilityRuleset, SimplifiedRepairabilityHeuristic,
    WashingMachineRuleset,
};
use crate::ruleset::Ruleset;

/// Every concrete ruleset known to this build, as base-trait references.
///
/// Used by CI checks (e.g. `expired_rulesets_have_superseded_by`) that must
/// iterate over all rulesets regardless of methodology. When a new ruleset is
/// added anywhere in `dpp-calc`, add a row here so the CI check covers it.
pub fn all_rulesets() -> &'static [&'static dyn Ruleset] {
    &[
        &SimplifiedRepairabilityHeuristic,
        &LaptopRuleset,
        &DisplaysRuleset,
        &WashingMachineRuleset,
        &CradleToGateRuleset,
    ]
}

/// Return the repairability ruleset in force for `product_category` on `on_date`.
///
/// Returns `None` when no ruleset covers the given category and date — either
/// the delegated act is not yet in force (stub with `from = 2100`) or the
/// category is not recognised.
pub fn resolve_repairability(
    product_category: &str,
    on_date: NaiveDate,
) -> Option<&'static dyn RepairabilityRuleset> {
    // One row per product-category × ruleset-version.
    // Add new versions by appending rows; the filter handles date selection.
    let all: &[(&str, &dyn RepairabilityRuleset)] = &[
        ("smartphone-tablet", &SimplifiedRepairabilityHeuristic),
        ("laptop", &LaptopRuleset),
        ("displays", &DisplaysRuleset),
        ("washing-machine", &WashingMachineRuleset),
    ];

    all.iter()
        .filter(|(cat, _)| *cat == product_category)
        .filter(|(_, r)| r.effective_dates().is_active_on(on_date))
        .map(|(_, r)| *r)
        .next()
}
