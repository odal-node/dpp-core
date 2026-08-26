//! [`LintFinding`] — one plausibility finding, and how findings are produced.
//!
//! A lint never gates publish. The severity is tone, not a gate.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::product_group::{self, DisclosureScope, ProductGroupData};

/// How strongly a lint finding should be read. Neither variant blocks
/// publish — the distinction is tone, not gating. Mirrors
/// [`dpp_rules::lint::LintSeverity`] in an owned, serialisable form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum LintSeverity {
    Warning,
    Notice,
}

/// A single plausibility finding. Mirrors [`dpp_rules::lint::LintFinding`] in
/// an owned, serialisable form.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LintFinding {
    pub code: String,
    pub field: String,
    pub severity: LintSeverity,
    pub message: String,
}

fn convert(f: dpp_rules::lint::LintFinding) -> LintFinding {
    LintFinding {
        code: f.code.to_owned(),
        field: f.field.to_owned(),
        severity: match f.severity {
            dpp_rules::lint::LintSeverity::Warning => LintSeverity::Warning,
            dpp_rules::lint::LintSeverity::Notice => LintSeverity::Notice,
        },
        message: f.message,
    }
}

/// Dispatch to the product group-specific lint pack. ProductGroups with no lint pack yet
/// (everything but battery/textile/unsold-goods in the first ruleset)
/// produce no findings.
#[must_use]
pub fn lint_product_group_data(data: &ProductGroupData, as_of: DateTime<Utc>) -> Vec<LintFinding> {
    match data {
        ProductGroupData::Battery(b) => {
            let cathode: Vec<f64> = b
                .cathode_material
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|m| m.weight_pct)
                .collect();
            let anode: Vec<f64> = b
                .anode_material
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|m| m.weight_pct)
                .collect();
            let electrolyte: Vec<f64> = b
                .electrolyte_material
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|m| m.weight_pct)
                .collect();
            let input = dpp_rules::lint::battery::BatteryLintInput {
                nominal_voltage_v: b.nominal_voltage_v,
                nominal_capacity_ah: b.nominal_capacity_ah,
                rated_energy_wh: b.rated_energy_wh,
                rated_capacity_kwh: b.rated_capacity_kwh,
                operating_temp_min_c: b.operating_temp_min_c,
                operating_temp_max_c: b.operating_temp_max_c,
                manufacturing_date_unix: b.manufacturing_date.map(|d| d.timestamp()),
                as_of_unix: as_of.timestamp(),
                cathode_material_pct: &cathode,
                anode_material_pct: &anode,
                electrolyte_material_pct: &electrolyte,
            };
            dpp_rules::lint::battery::lint_battery(&input)
                .into_iter()
                .map(convert)
                .collect()
        }
        ProductGroupData::Textile(t) => {
            let fibres: Vec<&str> = t
                .fibre_composition
                .iter()
                .map(|f| f.fibre.as_str())
                .collect();
            let input = dpp_rules::lint::textile::TextileLintInput {
                durability_score: t.durability_score,
                expected_wash_cycles: t.expected_wash_cycles,
                repair_count: t.repair_count,
                repair_history_url: t.repair_history_url.as_deref(),
                prior_use_cycles: t.prior_use_cycles,
                reuse_condition: t.reuse_condition.as_deref(),
                repair_score: t.repair_score,
                disassembly_instructions: t.disassembly_instructions.as_deref(),
                spare_parts_available: t.spare_parts_available,
                microplastic_shedding_mg_per_wash: t.microplastic_shedding_mg_per_wash,
                fibres: &fibres,
            };
            dpp_rules::lint::textile::lint_textile(&input)
                .into_iter()
                .map(convert)
                .collect()
        }
        ProductGroupData::UnsoldGoods(u) => {
            let lines: Vec<dpp_rules::lint::unsold_goods::DisclosureLineInput<'_>> = u
                .lines
                .iter()
                .map(|l| dpp_rules::lint::unsold_goods::DisclosureLineInput {
                    // A line may carry several CN codes (Annex I note (f)); the
                    // depth rule is about the first, which is the one the line
                    // is filed under.
                    cn_category: l
                        .cn_categories
                        .first()
                        .map_or("", product_group::CnCategory::as_str),
                    reason_point: l.reason.article_2_point(),
                    units: l.units_discarded.value,
                    weight_kg: l.weight_kg.value,
                    preparing_for_reuse_pct: l.treatment.preparing_for_reuse_pct,
                    recycling_pct: l.treatment.recycling_pct,
                    other_recovery_pct: l.treatment.other_recovery_pct,
                    disposal_pct: l.treatment.disposal_pct,
                    unknown_pct: l.treatment.unknown_pct,
                })
                .collect();
            let input = dpp_rules::lint::unsold_goods::UnsoldGoodsLintInput {
                lines: &lines,
                consolidated_undertaking_count: match &u.entity.scope {
                    DisclosureScope::Consolidated { undertakings } => Some(undertakings.len()),
                    DisclosureScope::Standalone => None,
                },
                measures_taken_len: u.measures_taken.trim().chars().count(),
                measures_planned_len: u.measures_planned.trim().chars().count(),
            };
            dpp_rules::lint::unsold_goods::lint_unsold_goods(&input)
                .into_iter()
                .map(convert)
                .collect()
        }
        _ => Vec::new(),
    }
}
