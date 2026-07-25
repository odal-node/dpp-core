//! Machine-readable calculator status map.
//!
//! The authoritative answer to "which sector–methodology metrics can be computed
//! today vs. which are awaiting a delegated act?" — consumed by the CLI
//! `odal calc status` view and by integrations doing compliance-readiness checks.
//!
//! **Status is derived, never declared.** Each entry names the ruleset that
//! implements it; whether that ruleset is in force comes from the ruleset's own
//! [`Effectivity`]. An earlier version of this file restated "pending" in a
//! second place, hand-maintained alongside the rulesets themselves — two
//! sources for one fact, free to drift apart.

use chrono::NaiveDate;

use super::resolve::all_rulesets;
use crate::ruleset::Effectivity;

/// What implements a sector–methodology pair, if anything.
#[derive(Debug, Clone, PartialEq)]
pub enum CalculatorImpl {
    /// Implemented by the ruleset with this id. Whether it is in force is read
    /// from that ruleset and never restated here.
    Ruleset(&'static str),
    /// The regulation imposes reporting or prohibition duties but mandates no
    /// quantitative methodology — there is nothing to calculate.
    ReportingOnly,
    /// A methodology exists in law but no ruleset is implemented yet. Distinct
    /// from `ReportingOnly`: this one is work outstanding, not work absent.
    NotImplemented,
}

/// Whether a calculator can be used for a product whose law was fixed on a
/// given date.
#[derive(Debug, Clone, PartialEq)]
pub enum CalculatorStatus {
    /// Implemented and in force on the queried date.
    Active { ruleset_id: &'static str },
    /// Implemented, in force, but only from a later date than the one queried.
    NotYetEffective {
        ruleset_id: &'static str,
        from: NaiveDate,
    },
    /// Implemented, but the instrument that would date it has not entered into
    /// force, so it has no application date at all.
    AwaitingInstrument {
        ruleset_id: &'static str,
        empowerment: &'static str,
    },
    /// Implemented, but its effective period has ended.
    Expired {
        ruleset_id: &'static str,
        until: NaiveDate,
    },
    /// No ruleset is implemented for this methodology.
    NotImplemented,
    /// No quantitative methodology is mandated.
    ReportingOnly,
    /// The entry names a ruleset id that no longer exists in `all_rulesets()`.
    /// Surfaced rather than panicked so a status view degrades loudly instead of
    /// dying; the drift guard in the test module fails on it.
    UnknownRuleset { ruleset_id: &'static str },
}

pub struct SectorCalculatorEntry {
    /// Sector key from the `SectorCatalog` (e.g. `"electronics"`, `"battery"`).
    pub sector_key: &'static str,
    /// Product category within the sector (e.g. `"smartphone-tablet"`).
    pub product_category: &'static str,
    /// Methodology identifier (e.g. `"repairability-heuristic"`, `"co2e-pef"`).
    pub methodology: &'static str,
    /// What implements this pair. Status is derived from it.
    pub implementation: CalculatorImpl,
}

impl SectorCalculatorEntry {
    /// Status for a product whose governing law was fixed on `law_in_force_on`.
    ///
    /// Takes the date rather than reading the clock: a readiness view for
    /// "today" and a readiness view for a product placed on the market in 2030
    /// are different questions, and the caller knows which it is asking.
    #[must_use]
    pub fn status_on(&self, law_in_force_on: NaiveDate) -> CalculatorStatus {
        let ruleset_id = match self.implementation {
            CalculatorImpl::ReportingOnly => return CalculatorStatus::ReportingOnly,
            CalculatorImpl::NotImplemented => return CalculatorStatus::NotImplemented,
            CalculatorImpl::Ruleset(id) => id,
        };

        let Some(ruleset) = all_rulesets().iter().find(|r| r.id().0 == ruleset_id) else {
            return CalculatorStatus::UnknownRuleset { ruleset_id };
        };

        match ruleset.effectivity() {
            Effectivity::Pending { empowerment, .. } => CalculatorStatus::AwaitingInstrument {
                ruleset_id,
                empowerment,
            },
            Effectivity::InForce { from, until } => {
                if law_in_force_on < *from {
                    CalculatorStatus::NotYetEffective {
                        ruleset_id,
                        from: *from,
                    }
                } else if let Some(until) = until
                    && law_in_force_on > *until
                {
                    CalculatorStatus::Expired {
                        ruleset_id,
                        until: *until,
                    }
                } else {
                    CalculatorStatus::Active { ruleset_id }
                }
            }
        }
    }
}

/// Complete map of all sector–methodology–implementation triples known to this
/// build.
///
/// Suitable for CLI status displays, API responses, and automated
/// compliance-readiness checks. Call [`SectorCalculatorEntry::status_on`] to
/// resolve an entry against a date.
pub fn sector_calculator_map() -> &'static [SectorCalculatorEntry] {
    &[
        // ── Electronics ──────────────────────────────────────────────────────
        SectorCalculatorEntry {
            sector_key: "electronics",
            product_category: "smartphone-tablet",
            // Non-regulatory: a simplified repairability heuristic is available,
            // NOT the enacted EU 2023/1669 Annex IV index, which is not
            // implemented. Output is a heuristic band, not a class.
            methodology: "repairability-heuristic",
            implementation: CalculatorImpl::Ruleset("repairability-heuristic-v1"),
        },
        SectorCalculatorEntry {
            sector_key: "electronics",
            product_category: "laptop",
            methodology: "repairability-heuristic",
            implementation: CalculatorImpl::Ruleset("laptop-repairability"),
        },
        SectorCalculatorEntry {
            sector_key: "electronics",
            product_category: "displays",
            methodology: "repairability-heuristic",
            implementation: CalculatorImpl::Ruleset("displays-repairability"),
        },
        SectorCalculatorEntry {
            sector_key: "electronics",
            product_category: "washing-machine",
            methodology: "repairability-heuristic",
            implementation: CalculatorImpl::Ruleset("washing-machine-repairability"),
        },
        // ── Battery ──────────────────────────────────────────────────────────
        SectorCalculatorEntry {
            sector_key: "battery",
            product_category: "all",
            methodology: "co2e-pef",
            implementation: CalculatorImpl::Ruleset("co2e-cradle-to-gate"),
        },
        SectorCalculatorEntry {
            sector_key: "battery",
            product_category: "all",
            methodology: "co2e-battery-regulation-art7",
            // No CfbRuleset impl exists — gated on the Art. 7(1) methodology
            // delegated act and a licensed factor dataset. See co2e::cfb.
            implementation: CalculatorImpl::NotImplemented,
        },
        // ── Unsold goods ─────────────────────────────────────────────────────
        SectorCalculatorEntry {
            sector_key: "unsoldGoods",
            product_category: "all",
            methodology: "unsold-goods-reporting",
            // ESPR Art. 25 imposes reporting/prohibition obligations only.
            implementation: CalculatorImpl::ReportingOnly,
        },
    ]
}
