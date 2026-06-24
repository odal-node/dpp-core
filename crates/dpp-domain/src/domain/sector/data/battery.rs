//! Battery (EU Battery Regulation 2023/1542).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::domain::gtin::Gtin;
use crate::domain::sector::enums::{BatteryChemistry, BatteryType, CarbonFootprintClass};

use super::shared::CriticalRawMaterial;

/// Battery-specific fields required by the EU Battery Regulation 2023/1542.
///
/// All `Option` fields are optional under the regulation; non-`Option` fields
/// are mandatory for publishing a battery DPP. Fields added in v2.0.0 of the
/// schema are marked `Option` and `skip_serializing_if` to maintain backward
/// compatibility with v1.0.0 data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BatteryData {
    // ── v1.0.0 mandatory fields ──────────────────────────────────────────
    /// 14-digit Global Trade Item Number identifying the battery model.
    pub gtin: Gtin,
    /// Battery electrochemical chemistry.
    pub battery_chemistry: BatteryChemistry,
    /// Nominal voltage in volts.
    pub nominal_voltage_v: f64,
    /// Nominal capacity in ampere-hours.
    pub nominal_capacity_ah: f64,
    /// Expected lifetime in full charge–discharge cycles.
    pub expected_lifetime_cycles: u32,
    /// Carbon footprint in kg CO₂e per battery unit (manufacturer-supplied or calculated).
    pub co2e_per_unit_kg: f64,

    // ── v1.0.0 optional fields ───────────────────────────────────────────
    /// Recycled cobalt content as a percentage of total cobalt (0.0–100.0).
    pub recycled_content_cobalt_pct: Option<f64>,
    /// Recycled lithium content as a percentage of total lithium (0.0–100.0).
    pub recycled_content_lithium_pct: Option<f64>,
    /// Recycled nickel content as a percentage of total nickel (0.0–100.0).
    pub recycled_content_nickel_pct: Option<f64>,
    /// Current state of health as a percentage of original rated capacity.
    pub state_of_health_pct: Option<f64>,
    /// Rated energy in kilowatt-hours (distinct from capacity in Ah).
    pub rated_capacity_kwh: Option<f64>,

    // ── v2.0.0 — Annex XIII compliance fields (Battery Reg. 2023/1542) ──
    /// Carbon footprint performance class (A–E) per Battery Regulation Art. 7(2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub carbon_footprint_class: Option<CarbonFootprintClass>,

    /// URL to supply chain due diligence documentation (Art. 47–52).
    /// Must link to a publicly accessible policy describing the due
    /// diligence process for raw material sourcing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub due_diligence_url: Option<String>,

    /// Cathode active material composition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cathode_material: Option<Vec<MaterialComposition>>,

    /// Anode active material composition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anode_material: Option<Vec<MaterialComposition>>,

    /// Electrolyte composition.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub electrolyte_material: Option<Vec<MaterialComposition>>,

    /// Critical raw materials present (Art. 5(2)) — list of CAS or EC numbers.
    /// The EU Critical Raw Materials Act (2024/1252) defines the canonical list.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_raw_materials: Option<Vec<CriticalRawMaterial>>,

    /// URL or text for disassembly / dismantling instructions (Annex XIII §6).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disassembly_instructions_url: Option<String>,

    /// State-of-health determination methodology identifier, e.g.
    /// `"IEC 62660-1:2018"` or `"proprietary:vendor-model-v3"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub soh_methodology: Option<String>,

    /// Minimum operating temperature in °C.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_temp_min_c: Option<f64>,

    /// Maximum operating temperature in °C.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub operating_temp_max_c: Option<f64>,

    /// Rated energy in watt-hours (Wh). Required by Annex XIII separately
    /// from `rated_capacity_kwh`. For cells this is the Wh stamping value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rated_energy_wh: Option<f64>,

    /// Recycled lead content as a percentage (for lead-acid batteries).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycled_content_lead_pct: Option<f64>,

    /// Weight of the battery in kilograms.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_weight_kg: Option<f64>,

    /// Battery type category per EU Battery Regulation 2023/1542 Art. 2.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_type: Option<BatteryType>,

    /// Round-trip energy efficiency at 50% state of charge (percentage).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub round_trip_efficiency_pct: Option<f64>,

    /// Internal resistance in milliohms (mΩ) at 50% SoC.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub internal_resistance_mohm: Option<f64>,

    // ── v2.0.0 — Annex XIII identity & origin fields (Battery Reg. 2023/1542) ─
    /// Date and time of manufacture (Annex XIII §2 — "date of manufacture").
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturing_date: Option<DateTime<Utc>>,

    /// Plant / location of manufacture (ISO 3166-1 alpha-2 country code or
    /// "ISO country:city" free-text per Annex XIII §2).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub manufacturing_place: Option<String>,

    /// Manufacturer's battery model identifier as it appears on the physical label
    /// or accompanying technical documentation (Annex XIII §1).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_model_id: Option<String>,

    /// Unique battery passport identifier issued at commissioning.
    /// Format: per the Commission's implementing act on the battery passport
    /// (expected ~2026); until then a UUID v4 is accepted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battery_passport_number: Option<String>,
}

/// Material composition entry for cathode, anode, or electrolyte.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MaterialComposition {
    /// Chemical name or formula, e.g. `"LiFePO4"`, `"graphite"`, `"LiPF6"`.
    pub name: String,
    /// Weight percentage in the component (0.0–100.0).
    pub weight_pct: f64,
    /// CAS Registry Number if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cas_number: Option<String>,
}
