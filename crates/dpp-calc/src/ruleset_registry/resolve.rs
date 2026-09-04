//! `all_rulesets()` plus the per-methodology date-based `resolve_*` lookups.

use chrono::NaiveDate;

use crate::assessability::Assessability;
use crate::co2e::CradleToGateRuleset;
use crate::recycled_content::{Art8Phase1Ruleset, Art8Phase2Ruleset, RecycledContentRuleset};
use crate::repairability::thresholds::{
    DisplaysRuleset, LaptopRuleset, RepairabilityRuleset, SimplifiedRepairabilityHeuristic,
    WashingMachineRuleset,
};
use crate::repairability_index::{Eu2023_1669Ruleset, RepairabilityIndexRuleset};
use crate::ruleset::{Effectivity, Ruleset};

/// Every concrete ruleset known to this build, as base-trait references.
///
/// Used by CI checks (e.g. `expired_rulesets_have_superseded_by`) that must
/// iterate over all rulesets regardless of methodology. When a new ruleset is
/// added anywhere in `dpp-calc`, add a row here so the CI check covers it.
///
/// Forgetting is caught rather than trusted:
/// `every_ruleset_impl_reaches_the_registry` scans this crate for
/// `impl Ruleset` and fails on any type missing from this list. That matters
/// because two safety properties are asserted by *iterating* it — the
/// `ParameterBasis::Sourced` tripwire and the empty-parameters check — so a
/// ruleset nobody registers escapes both while looking entirely normal.
pub fn all_rulesets() -> &'static [&'static dyn Ruleset] {
    &[
        &SimplifiedRepairabilityHeuristic,
        &LaptopRuleset,
        &DisplaysRuleset,
        &WashingMachineRuleset,
        &CradleToGateRuleset,
        &Eu2023_1669Ruleset,
        &Art8Phase1Ruleset,
        &Art8Phase2Ruleset,
    ]
}

/// Return the Art. 8 recycled-content ruleset governing `product_category` for a
/// battery placed on the EU market on `law_in_force_on`, or the reason there
/// isn't one.
///
/// `product_category` is the Art. 8 scope bucket, not the passport's
/// `batteryType`: map one to the other with
/// [`art8_category_for`](dpp_rules::batteries::recycled_content::art8_category_for),
/// which needs the energy capacity as well as the type because the industrial
/// limb is scoped at 2 kWh. The two keys this table answers are
/// `"industrial-ev-sli"` and `"lmt"`; a portable battery is
/// [`OutOfScope`](Assessability::OutOfScope), which Art. 8 genuinely is for it.
///
/// The LMT rows are the reason this cannot be a date comparison alone. Art. 8(2)
/// does not name LMT batteries, so an LMT battery placed on the market in 2032
/// resolves to `NotYetInForce { applies_from: 2036-08-18 }` — not to Art. 8(2)
/// with an empty shortfall list, which would report a rule as satisfied that
/// never applied.
pub fn resolve_recycled_content(
    product_category: &str,
    law_in_force_on: NaiveDate,
) -> Assessability<&'static dyn RecycledContentRuleset> {
    let all: &[(&str, &'static dyn RecycledContentRuleset)] = &[
        ("industrial-ev-sli", &Art8Phase1Ruleset),
        ("industrial-ev-sli", &Art8Phase2Ruleset),
        // Art. 8(3) is the first provision to reach LMT batteries — there is
        // deliberately no Phase 1 row here.
        ("lmt", &Art8Phase2Ruleset),
    ];

    let rows = || all.iter().filter(|(cat, _)| *cat == product_category);

    if let Some((_, r)) = rows().find(|(_, r)| r.effectivity().is_active_on(law_in_force_on)) {
        return Assessability::Assessed(*r);
    }

    // Before any phase binds, the actionable answer is the soonest start date;
    // after the last one there is none, because Phase 2 is open-ended.
    let mut soonest: Option<(&'static str, NaiveDate)> = None;
    for (_, r) in rows() {
        if let Effectivity::InForce { from, .. } = r.effectivity()
            && law_in_force_on < *from
            && soonest.is_none_or(|(_, best)| *from < best)
        {
            soonest = Some((r.id().0, *from));
        }
    }

    match soonest {
        Some((ruleset_id, applies_from)) => Assessability::NotYetInForce {
            ruleset_id,
            applies_from,
        },
        None => Assessability::OutOfScope,
    }
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

/// Return the EU 2023/1669 repairability-index ruleset governing
/// `product_category` for a product whose law was fixed on `law_in_force_on`,
/// or the reason there isn't one.
///
/// The enacted index is a different methodology from the non-regulatory
/// heuristic behind [`resolve_repairability`] — different parameters, a 1–5
/// scale and its own class boundaries — so it gets its own resolver rather than
/// overloading that one.
pub fn resolve_repairability_index(
    product_category: &str,
    law_in_force_on: NaiveDate,
) -> Assessability<&'static dyn RepairabilityIndexRuleset> {
    // Reg. (EU) 2023/1669 covers smartphones and slate tablets.
    let all: &[(&str, &'static dyn RepairabilityIndexRuleset)] =
        &[("smartphone-tablet", &Eu2023_1669Ruleset)];

    let rows = || all.iter().filter(|(cat, _)| *cat == product_category);

    if let Some((_, r)) = rows().find(|(_, r)| r.effectivity().is_active_on(law_in_force_on)) {
        return Assessability::Assessed(*r);
    }
    match rows().next() {
        None => Assessability::OutOfScope,
        Some((_, r)) => match r.effectivity() {
            Effectivity::Pending { empowerment, .. } => Assessability::Undetermined {
                ruleset_id: r.id().0,
                empowerment,
            },
            Effectivity::InForce { from, .. } => Assessability::NotYetInForce {
                ruleset_id: r.id().0,
                applies_from: *from,
            },
        },
    }
}
