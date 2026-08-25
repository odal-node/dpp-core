//! Mattress (EU ESPR Working Plan 2025-2030, indicative adoption 2029).

use serde::{Deserialize, Serialize};

use crate::domain::gtin::Gtin;

use super::common::SvhcSubstance;

/// Mattress product-group data for EU ESPR DPP compliance.
///
/// # Why this is not a furniture category
///
/// ESPR Art. 18's statutory priority list reads "furniture, **including**
/// mattresses", and mattresses were carried here as a `productType` value on
/// [`FurnitureData`](super::furniture::FurnitureData). The 2025-2030 working
/// plan then selected the two separately — different ranking, different
/// indicative adoption year — so they will be regulated by different delegated
/// acts on different timetables, and a passport cannot be governed by "half of
/// furniture's act".
///
/// The grouping changing between a framework and its own working plan is the
/// clearest available evidence that product group is a decision each act makes,
/// not a taxonomy that can be fixed once and read off.
///
/// # Why the fields are furniture's
///
/// This is furniture's field set with `product_type` removed — a mattress is the
/// product type — and **nothing added**. No delegated act exists for mattresses,
/// so any mattress-specific field (core construction, firmness, fire-retardant
/// treatment) would be invented rather than read from a text. They can be added
/// when there is something to read.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct MattressData {
    /// 14-digit GTIN identifying the mattress.
    pub gtin: Gtin,
    /// Primary material, e.g. `"solid-wood"`, `"metal"`, `"upholstered"`, `"mixed"`.
    pub primary_material: String,
    /// ISO 3166-1 alpha-2 country of manufacture.
    pub country_of_origin: String,

    /// Carbon footprint in kg CO₂e per unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub co2e_per_unit_kg: Option<f64>,
    /// Recycled content as a percentage of total weight (0.0–100.0).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycled_content_pct: Option<f64>,
    /// Repairability score (0.0–10.0, non-regulatory heuristic — not EN 45554 / EU 2023/1669).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repairability_score: Option<f64>,
    /// SVHC substances present above 0.1% w/w per REACH Article 33.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub svhc_substances: Option<Vec<SvhcSubstance>>,
    /// URL to disassembly / deconstruction instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub disassembly_instructions_url: Option<String>,
    /// End-of-life disposal or recycling instructions.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_of_life_instructions: Option<String>,
}
