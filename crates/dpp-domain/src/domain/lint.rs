//! Passport plausibility lint dispatch — maps [`SectorData`] onto the
//! `dpp-rules::lint` pack and carries the owned, serialisable wire types the
//! engine persists on [`crate::domain::passport::Passport::lint_result`].
//!
//! Unlike [`crate::ports::compliance`], there is no pluggable strategy here:
//! the lint pack ships directly in `dpp-rules` and is not an extension seam.

use chrono::{DateTime, Datelike, Utc};
use serde::{Deserialize, Serialize};

use super::sector::{SectorData, UnsoldGoodsDestination};

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
/// sector data. Never gates publish — see
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
    pub fn compute(data: &SectorData) -> Self {
        let now = Utc::now();
        Self {
            pack_version: dpp_rules::lint::LINT_PACK_VERSION.to_owned(),
            findings: lint_sector_data(data, now),
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

fn unsold_goods_destination_code(d: &UnsoldGoodsDestination) -> &'static str {
    match d {
        UnsoldGoodsDestination::Donation => "donation",
        UnsoldGoodsDestination::Recycling => "recycling",
        UnsoldGoodsDestination::Repurposing => "repurposing",
        UnsoldGoodsDestination::SupplierReturn => "supplier_return",
        UnsoldGoodsDestination::ExemptDestruction => "exempt_destruction",
    }
}

/// Dispatch to the sector-specific lint pack. Sectors with no lint pack yet
/// (everything but battery/textile/unsold-goods in the first ruleset)
/// produce no findings.
#[must_use]
pub fn lint_sector_data(data: &SectorData, as_of: DateTime<Utc>) -> Vec<LintFinding> {
    match data {
        SectorData::Battery(b) => {
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
        SectorData::Textile(t) => {
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
        SectorData::UnsoldGoods(u) => {
            let input = dpp_rules::lint::unsold_goods::UnsoldGoodsLintInput {
                reporting_period: &u.reporting_period,
                volume_kg: u.volume_kg,
                destination: unsold_goods_destination_code(&u.destination),
                operator_name: u.operator_name.as_deref(),
                destruction_justification: u.destruction_justification.as_deref(),
                as_of_year: as_of.year().max(0) as u32,
                as_of_month: as_of.month(),
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
    use crate::domain::gtin::Gtin;
    use crate::domain::sector::{
        BatteryChemistry, BatteryData, UnsoldGoodsReason, UnsoldGoodsReport,
    };

    fn battery() -> BatteryData {
        BatteryData {
            gtin: Gtin::parse("09506000134352").unwrap(),
            battery_chemistry: BatteryChemistry::Lfp,
            nominal_voltage_v: 3.7,
            nominal_capacity_ah: 10.0,
            expected_lifetime_cycles: 500,
            co2e_per_unit_kg: 5.0,
            recycled_content_cobalt_pct: None,
            recycled_content_lithium_pct: None,
            recycled_content_nickel_pct: None,
            state_of_health_pct: None,
            rated_capacity_kwh: None,
            carbon_footprint_class: None,
            due_diligence_url: None,
            cathode_material: None,
            anode_material: None,
            electrolyte_material: None,
            critical_raw_materials: None,
            disassembly_instructions_url: None,
            soh_methodology: None,
            operating_temp_min_c: None,
            operating_temp_max_c: None,
            rated_energy_wh: Some(37.0),
            recycled_content_lead_pct: None,
            battery_weight_kg: None,
            battery_type: None,
            round_trip_efficiency_pct: None,
            internal_resistance_mohm: None,
            manufacturing_date: None,
            manufacturing_place: None,
            battery_model_id: None,
            battery_passport_number: None,
        }
    }

    #[test]
    fn clean_battery_produces_no_findings() {
        let data = SectorData::Battery(battery());
        assert!(lint_sector_data(&data, Utc::now()).is_empty());
    }

    #[test]
    fn battery_energy_mismatch_surfaces_as_domain_finding() {
        let mut b = battery();
        b.rated_energy_wh = Some(500.0); // 3.7 * 10.0 = 37.0 expected
        let data = SectorData::Battery(b);
        let findings = lint_sector_data(&data, Utc::now());
        assert_eq!(findings.len(), 1);
        assert_eq!(findings[0].code, "battery.energy_capacity_mismatch");
        assert_eq!(findings[0].severity, LintSeverity::Notice);
    }

    #[test]
    fn unsold_goods_third_party_without_operator_name_triggers() {
        let report = UnsoldGoodsReport {
            reporting_period: "2026-Q2".into(),
            volume_kg: 500.0,
            product_category: "apparel".into(),
            reason: UnsoldGoodsReason::EndOfSeason,
            destination: UnsoldGoodsDestination::Donation,
            destruction_justification: None,
            country_of_disposal: "MK".into(),
            operator_name: None,
        };
        let data = SectorData::UnsoldGoods(report);
        let findings = lint_sector_data(&data, Utc::now());
        assert!(
            findings
                .iter()
                .any(|f| f.code == "unsold_goods.operator_name_missing_for_third_party_destination")
        );
    }

    #[test]
    fn other_sector_produces_no_findings() {
        let data = SectorData::Other(serde_json::json!({"sector": "toy"}));
        assert!(lint_sector_data(&data, Utc::now()).is_empty());
    }

    #[test]
    fn lint_result_compute_stamps_pack_version_and_timestamp() {
        let data = SectorData::Battery(battery());
        let result = LintResult::compute(&data);
        assert_eq!(result.pack_version, dpp_rules::lint::LINT_PACK_VERSION);
        assert!(result.findings.is_empty());
    }

    #[test]
    fn lint_result_serde_round_trip() {
        let data = SectorData::Battery(battery());
        let result = LintResult::compute(&data);
        let json = serde_json::to_value(&result).unwrap();
        let back: LintResult = serde_json::from_value(json).unwrap();
        assert_eq!(back, result);
    }
}
