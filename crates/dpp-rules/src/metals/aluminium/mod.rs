//! Aluminium cross-field rules — EU ESPR + CBAM (EU Regulation 2023/956).
//!
//! ## Regulatory status
//! No delegated act requires an aluminium passport yet; the ESPR Working Plan
//! 2025-2030 (COM(2025) 187) gives aluminium an indicative adoption year of 2027.
//! CBAM (EU 2023/956) covers
//! embedded-carbon reporting for aluminium imports but does **not** set
//! production-level CO₂e thresholds that would make a product non-compliant for
//! DPP purposes. The thresholds below are **industry/CBAM benchmark values**
//! used by the `product_group-aluminium` plugin; they are not finalized EU DPP mandates.
//!
//! ## Production routes and CO₂e reference thresholds (kg CO₂e / tonne Al)
//! | Route                | Threshold used by plugin |
//! |----------------------|--------------------------|
//! | `primary`            | ≤ 10 000                 |
//! | `secondary-recycled` | ≤ 1 000                  |
//! | `mixed`              | ≤ 5 000                  |
//!
//! Source: CBAM benchmark values / `product_group-aluminium` plugin. These are
//! informational reference points until a finalized EU DPP threshold is adopted.
//!
//! ## Schema fields (aluminium v1.1.0)
//! - `alloyGrade`         — free-form string (e.g. `"1xxx"`, `"6061"`)
//! - `productionRoute`    — enum: `primary | secondary-recycled | mixed`
//! - `co2ePerTonneKg`     — kg CO₂e per tonne, non-negative
//! - `recycledContentPct` — 0–100 %
//! - `countryOfOrigin`    — ISO 3166-1 alpha-2

mod co2e_threshold;
#[cfg(test)]
mod tests;

pub use co2e_threshold::*;
