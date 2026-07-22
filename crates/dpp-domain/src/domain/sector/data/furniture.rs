//! Furniture (EU ESPR Working Plan 2025-2030, mandate ~2028-2031).

use serde::{Deserialize, Serialize};

use crate::domain::gtin::Gtin;

use super::shared::SvhcSubstance;

/// Furniture sector data for EU ESPR DPP compliance.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FurnitureData {
    /// 14-digit GTIN identifying the furniture product.
    pub gtin: Gtin,
    /// Product type, e.g. `"chair"`, `"table"`, `"sofa"`, `"mattress"`, `"shelf"`, `"other"`.
    pub product_type: String,
    /// Primary material, e.g. `"solid-wood"`, `"engineered-wood"`, `"metal"`, `"upholstered"`, `"mixed"`.
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
