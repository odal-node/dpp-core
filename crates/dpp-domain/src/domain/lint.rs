//! Passport plausibility lint dispatch — maps [`ProductGroupData`] onto the
//! `dpp-rules::lint` pack and carries the owned, serialisable wire types the
//! engine persists on [`crate::domain::passport::Passport::lint_result`].
//!
//! Unlike [`crate::ports::compliance`], there is no pluggable strategy here:
//! the lint pack ships directly in `dpp-rules` and is not an extension seam.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::product_group::{DisclosureScope, ProductGroupData};

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

/// The result of running the plausibility lint pack against a passport's
/// product group data. Never gates publish — see
/// [`crate::domain::passport::Passport::lint_result`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LintResult {
    /// The `dpp_rules::lint::LINT_PACK_VERSION` that produced `findings`.
    pub pack_version: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub findings: Vec<LintFinding>,
    pub assessed_at: DateTime<Utc>,
}

impl LintResult {
    /// Run the plausibility lint pack against `data`, stamping `assessed_at`
    /// as `Utc::now()`.
    #[must_use]
    pub fn compute(data: &ProductGroupData) -> Self {
        let now = Utc::now();
        Self {
            pack_version: dpp_rules::lint::LINT_PACK_VERSION.to_owned(),
            findings: lint_product_group_data(data, now),
            assessed_at: now,
        }
    }
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
                        .map_or("", super::product_group::CnCategory::as_str),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::product_group::BatteryData;
    use crate::domain::product_group::data::unsold_goods::{CnCategory, DiscardReason};

    fn battery() -> BatteryData {
        BatteryData {
            nominal_voltage_v: 3.7,
            nominal_capacity_ah: 10.0,
            expected_lifetime_cycles: Some(500),
            co2e_per_unit_kg: 5.0,
            rated_energy_wh: Some(37.0),
            ..crate::test_support::sample_battery_data()
        }
    }

    #[test]
    fn clean_battery_produces_no_findings() {
        let data = ProductGroupData::Battery(Box::new(battery()));
        assert!(lint_product_group_data(&data, Utc::now()).is_empty());
    }

    #[test]
    fn battery_energy_mismatch_surfaces_as_domain_finding() {
        let mut b = battery();
        b.rated_energy_wh = Some(500.0); // 3.7 * 10.0 = 37.0 expected
        let data = ProductGroupData::Battery(Box::new(b));
        let findings = lint_product_group_data(&data, Utc::now());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "battery.energy_capacity_mismatch");
        assert_eq!(findings[0].severity, LintSeverity::Notice);
    }

    #[test]
    fn a_well_formed_disclosure_produces_no_findings() {
        let data = ProductGroupData::UnsoldGoods(crate::test_support::sample_unsold_goods_report());
        assert_eq!(lint_product_group_data(&data, Utc::now()), Vec::new());
    }

    /// Annex I note (i) provides `unknown` for the share that could not be
    /// established, so a split that does not reach 100 has lost weight rather
    /// than being unsure about it.
    #[test]
    fn a_treatment_split_that_misses_100_is_flagged() {
        let mut report = crate::test_support::sample_unsold_goods_report();
        report.lines[0].treatment.disposal_pct = 1;
        let data = ProductGroupData::UnsoldGoods(report);
        let findings = lint_product_group_data(&data, Utc::now());
        assert!(
            findings
                .iter()
                .any(|f| f.code == "unsold_goods.treatment_split_does_not_total_100"),
            "{findings:?}"
        );
    }

    /// Art. 3 requires four digits for Annex II products, and chapter 85 holds
    /// several — so a chapter-level line there hides which heading applied.
    #[test]
    fn a_chapter_holding_annex_ii_headings_is_flagged() {
        let mut report = crate::test_support::sample_unsold_goods_report();
        report.lines[0].cn_categories = vec![CnCategory::parse("85").expect("valid chapter")];
        let data = ProductGroupData::UnsoldGoods(report);
        let findings = lint_product_group_data(&data, Utc::now());
        assert!(
            findings
                .iter()
                .any(|f| f.code == "unsold_goods.cn_category_needs_four_digits"),
            "{findings:?}"
        );
    }

    /// Point (h) applies "only where none of the circumstances referred to in
    /// points (a) to (g) are applicable", so it cannot sit beside one of them
    /// for the same category.
    #[test]
    fn donation_claimed_beside_a_stronger_reason_is_flagged() {
        let mut report = crate::test_support::sample_unsold_goods_report();
        let mut second = report.lines[0].clone();
        second.reason = DiscardReason::OfferedForDonationNotAccepted;
        report.lines.push(second);
        let data = ProductGroupData::UnsoldGoods(report);
        let findings = lint_product_group_data(&data, Utc::now());
        assert!(
            findings
                .iter()
                .any(|f| f.code == "unsold_goods.donation_reason_alongside_stronger_reason"),
            "{findings:?}"
        );
    }

    #[test]
    fn other_product_group_produces_no_findings() {
        let data = ProductGroupData::other(serde_json::json!({"productGroup": "packaging"}))
            .expect("packaging has no typed variant");
        assert!(lint_product_group_data(&data, Utc::now()).is_empty());
    }

    #[test]
    fn lint_result_compute_stamps_pack_version_and_timestamp() {
        let data = ProductGroupData::Battery(Box::new(battery()));
        let result = LintResult::compute(&data);
        assert_eq!(result.pack_version, dpp_rules::lint::LINT_PACK_VERSION);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn lint_result_serde_round_trip() {
        let data = ProductGroupData::Battery(Box::new(battery()));
        let result = LintResult::compute(&data);
        let json = serde_json::to_value(&result).unwrap();
        let back: LintResult = serde_json::from_value(json).unwrap();
        assert_eq!(back, result);
    }
}
