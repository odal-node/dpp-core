//! `all_rulesets()` plus the per-methodology date-based `resolve_*` lookups.

use chrono::NaiveDate;

use crate::assessability::Assessability;
use crate::co2e::CradleToGateRuleset;
use crate::repairability::thresholds::{
    DisplaysRuleset, LaptopRuleset, RepairabilityRuleset, SimplifiedRepairabilityHeuristic,
    WashingMachineRuleset,
};
use crate::ruleset::{Effectivity, Ruleset};

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

/// Return the repairability ruleset governing `product_category` for a product
/// whose law was fixed on `law_in_force_on`, or the reason there isn't one.
///
/// The date is the product's regulated triggering event, never today — see
/// [`AssessmentClock`](crate::clock::AssessmentClock).
///
/// When nothing governs, the returned [`Assessability`] says which of the four
/// reasons applies. Where several rows exist for one category and none is
/// active, the reason is taken from the row closest to applying: a known future
/// start date first, then a ruleset awaiting an unadopted act, then an expired
/// one. A dated answer is more actionable than an undated one, and both beat
/// pointing at a rule that has already been replaced.
pub fn resolve_repairability(
    product_category: &str,
    law_in_force_on: NaiveDate,
) -> Assessability<&'static dyn RepairabilityRuleset> {
    // One row per product-category × ruleset-version.
    // Add new versions by appending rows; the filter handles date selection.
    let all: &[(&str, &'static dyn RepairabilityRuleset)] = &[
        ("smartphone-tablet", &SimplifiedRepairabilityHeuristic),
        ("laptop", &LaptopRuleset),
        ("displays", &DisplaysRuleset),
        ("washing-machine", &WashingMachineRuleset),
    ];

    let rows = || all.iter().filter(|(cat, _)| *cat == product_category);

    if let Some((_, r)) = rows().find(|(_, r)| r.effectivity().is_active_on(law_in_force_on)) {
        return Assessability::Assessed(*r);
    }

    let mut not_yet: Option<(&'static str, NaiveDate)> = None;
    let mut undetermined: Option<(&'static str, &'static str)> = None;
    let mut expired: Option<(&'static str, NaiveDate, Option<&'static str>)> = None;

    for (_, r) in rows() {
        let id = r.id().0;
        match r.effectivity() {
            Effectivity::Pending { empowerment, .. } => {
                undetermined.get_or_insert((id, empowerment));
            }
            Effectivity::InForce { from, until } => {
                if law_in_force_on < *from {
                    // Keep the soonest start date.
                    match not_yet {
                        Some((_, best)) if best <= *from => {}
                        _ => not_yet = Some((id, *from)),
                    }
                } else if let Some(until) = until {
                    // Keep the most recently ended.
                    match expired {
                        Some((_, best, _)) if best >= *until => {}
                        _ => expired = Some((id, *until, r.regulatory_basis().superseded_by)),
                    }
                }
            }
        }
    }

    if let Some((ruleset_id, applies_from)) = not_yet {
        Assessability::NotYetInForce {
            ruleset_id,
            applies_from,
        }
    } else if let Some((ruleset_id, empowerment)) = undetermined {
        Assessability::Undetermined {
            ruleset_id,
            empowerment,
        }
    } else if let Some((ruleset_id, until, superseded_by)) = expired {
        Assessability::Expired {
            ruleset_id,
            until,
            superseded_by,
        }
    } else {
        Assessability::OutOfScope
    }
}
