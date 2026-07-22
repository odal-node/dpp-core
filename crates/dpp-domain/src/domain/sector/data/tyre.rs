//! Tyres (EU ESPR Working Plan 2025-2030, mandate ~2029).

use serde::{Deserialize, Serialize};

use crate::domain::gtin::Gtin;

/// Tyre sector data for EU tyre labelling compliance.
///
/// Per EU Regulation 2020/740 (effective 1 May 2021, replacing 1222/2009).
/// Fuel efficiency and wet grip classes use the A–E scale (5 classes).
/// The old A–G scale from Regulation 1222/2009 was retired in May 2021.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TyreData {
    /// 14-digit GTIN identifying the tyre model.
    pub gtin: Gtin,
    /// Tyre class per EU 2020/740: `"C1"` (passenger cars), `"C2"` (vans/light trucks), `"C3"` (heavy trucks).
    pub tyre_class: String,
    /// Fuel efficiency class **A–E** per EU 2020/740 (A = lowest rolling resistance).
    /// NOTE: the old A–G scale (Regulation 1222/2009) was replaced on 1 May 2021.
    pub fuel_efficiency_class: String,
    /// Wet grip class **A–E** per EU 2020/740 (A = shortest stopping distance on wet road).
    pub wet_grip_class: String,
    /// External rolling noise in decibels (dB), measured per UN ECE R117.
    pub external_rolling_noise_db: f64,

    /// Noise performance class **A/B/C** per EU 2020/740 Annex I.
    /// A = significantly below the noise limit; B = below limit; C = at or above limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noise_performance_class: Option<String>,
    /// Rolling resistance coefficient in N/kN.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rolling_resistance_n_per_kn: Option<f64>,
    /// Recycled rubber content as a percentage of total rubber weight.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub recycled_rubber_pct: Option<f64>,
    /// Carbon footprint in kg CO₂e per tyre unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub co2e_per_tyre_kg: Option<f64>,
}
