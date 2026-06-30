//! Machine-readable calculator status map (rules Gap 7).
//!
//! The authoritative answer to "which sector–methodology metrics can be computed
//! today vs. which are awaiting a delegated act?" — consumed by the CLI
//! `odal calc status` view and by integrations doing compliance-readiness checks.

/// Whether a calculator is implemented and legally in force, or still awaiting
/// a delegated act.
#[derive(Debug, Clone, PartialEq)]
pub enum CalculatorStatus {
    /// Calculator is implemented and the underlying regulation is in force.
    Active {
        /// The `RulesetId` string that identifies the concrete implementation.
        /// Must match a `Ruleset::id()` in [`all_rulesets`](super::all_rulesets)
        /// (enforced by `active_map_entries_reference_real_rulesets`).
        ruleset_id: &'static str,
    },
    /// Delegated act not yet in force; a stub implementation exists with a
    /// sentinel effective date (`2100-01-01`) that blocks runtime use.
    PendingDelegatedAct {
        /// Approximate year the delegated act is expected, if known.
        expected_year: Option<u16>,
    },
    /// No calculator planned; the sector has reporting requirements only.
    ReportingOnly,
}

pub struct SectorCalculatorEntry {
    /// Sector key from the `SectorCatalog` (e.g. `"electronics"`, `"battery"`).
    pub sector_key: &'static str,
    /// Product category within the sector (e.g. `"smartphone-tablet"`).
    pub product_category: &'static str,
    /// Methodology identifier (e.g. `"repairability-heuristic"`, `"co2e-pef"`).
    pub methodology: &'static str,
    pub status: CalculatorStatus,
}

/// Complete map of all sector–methodology–status triples known to this build.
///
/// This is the machine-readable answer to "which rules are live vs. awaiting
/// a delegated act?" — suitable for CLI status displays, API responses, and
/// automated compliance-readiness checks.
pub fn sector_calculator_map() -> &'static [SectorCalculatorEntry] {
    &[
        // ── Electronics ──────────────────────────────────────────────────────
        SectorCalculatorEntry {
            sector_key: "electronics",
            product_category: "smartphone-tablet",
            // Non-regulatory: a simplified repairability heuristic is available
            // (Active = computable), NOT the enacted EU 2023/1669 Annex IV index,
            // which is not yet implemented. Output is a heuristic band, not a class.
            methodology: "repairability-heuristic",
            status: CalculatorStatus::Active {
                ruleset_id: "repairability-heuristic-v1",
            },
        },
        SectorCalculatorEntry {
            sector_key: "electronics",
            product_category: "laptop",
            methodology: "repairability-heuristic",
            status: CalculatorStatus::PendingDelegatedAct {
                expected_year: Some(2027),
            },
        },
        SectorCalculatorEntry {
            sector_key: "electronics",
            product_category: "displays",
            methodology: "repairability-heuristic",
            status: CalculatorStatus::PendingDelegatedAct {
                expected_year: None,
            },
        },
        // ── Battery ──────────────────────────────────────────────────────────
        SectorCalculatorEntry {
            sector_key: "battery",
            product_category: "all",
            methodology: "co2e-pef",
            status: CalculatorStatus::Active {
                // Must match CradleToGateRuleset::id() — see all_rulesets().
                ruleset_id: "co2e-cradle-to-gate",
            },
        },
        SectorCalculatorEntry {
            sector_key: "battery",
            product_category: "all",
            methodology: "co2e-battery-regulation-art7",
            // Art. 7 carbon footprint declaration delegated act pending.
            status: CalculatorStatus::PendingDelegatedAct {
                expected_year: None,
            },
        },
        // ── Unsold Goods ───────────────────────────────────────────────────────
        SectorCalculatorEntry {
            sector_key: "unsoldGoods",
            product_category: "all",
            methodology: "unsold-goods-reporting",
            // Art. 25 imposes reporting/prohibition obligations only — no
            // quantitative calculation methodology is mandated.
            status: CalculatorStatus::ReportingOnly,
        },
        // ── Household appliances ────────────────────────────────────────────────
        SectorCalculatorEntry {
            sector_key: "electronics",
            product_category: "washing-machine",
            methodology: "repairability-heuristic",
            status: CalculatorStatus::PendingDelegatedAct {
                expected_year: Some(2026),
            },
        },
    ]
}
