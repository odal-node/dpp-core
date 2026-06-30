//! Electronics (EU Electronics DPP, adopted 18 March 2026, effective 1 April 2026).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::sector::enums::EnergyEfficiencyClass;
use crate::domain::sector::metrics::RepairabilityScore;

use super::shared::{CriticalRawMaterial, SvhcSubstance};

/// Electronics sector data for EU Electronics DPP compliance.
///
/// Mandatory for AI servers, high-end PCBs, and foldable phones immediately;
/// broader consumer electronics (earphones, chargers) from 1 January 2027.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ElectronicsData {
    /// 14-digit GTIN identifying the product model.
    pub gtin: String,
    /// Product category, e.g. `"smartphone"`, `"laptop"`, `"tablet"`, `"monitor"`,
    /// `"tv"`, `"server"`, `"charger"`, `"earphone"`, `"other"`.
    pub product_category: String,
    /// EU energy label class (A–G) per Energy Labelling Regulation 2017/1369.
    pub energy_efficiency_class: EnergyEfficiencyClass,
    /// Whole-lifecycle carbon footprint in kg CO₂e per unit.
    pub co2e_per_unit_kg: f64,

    /// Repairability score (non-regulatory heuristic — not EN 45554 / EU 2023/1669).
    /// `overall` ≥ 6.0 = good; < 4.0 = fails minimum standard.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repairability_score: Option<RepairabilityScore>,
    /// Whether spare parts are commercially available from the manufacturer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub spare_parts_available: Option<bool>,
    /// URL to the repair manual or repair information portal.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repair_manual_url: Option<String>,
    /// URL to disassembly / dismantling instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disassembly_instructions_url: Option<String>,
    /// SVHC substances present above 0.1% w/w (REACH Art. 33).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svhc_substances: Option<Vec<SvhcSubstance>>,
    /// Whether the product complies with RoHS Directive 2011/65/EU.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rohs_compliant: Option<bool>,
    /// Critical raw materials present (EU CRM Act 2024/1252).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_raw_materials: Option<Vec<CriticalRawMaterial>>,
    /// Recycled content as a percentage of total product weight (0.0–100.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycled_content_pct: Option<f64>,
    /// Standby power consumption in watts.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub standby_power_w: Option<f64>,
    /// Expected product lifetime in years under normal use.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected_lifetime_years: Option<u32>,
    /// Date until which firmware / software updates are guaranteed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub firmware_update_until: Option<DateTime<Utc>>,
}
