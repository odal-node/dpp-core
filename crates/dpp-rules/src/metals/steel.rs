//! Steel cross-field rules — EU ESPR + CBAM (EU Regulation 2023/956).
//!
//! ## Regulatory status
//! No steel DPP mandate is currently in force. CBAM (EU 2023/956) requires
//! embedded-carbon reporting for steel imports but does **not** set per-tonne
//! CO₂e thresholds that would make a product non-compliant under a DPP regime.
//! Thresholds are expected to emerge from the ESPR delegated acts for steel,
//! timeline not yet published.
//!
//! ## Production routes (schema enum)
//! The steel schema v1.0.0 recognises three routes:
//! | Route key          | Process                           | Typical CO₂e (tCO₂e/t) |
//! |--------------------|-----------------------------------|--------------------------|
//! | `blast-furnace`    | BF-BOF (iron ore + O₂ converter)  | 1.8 – 2.5                |
//! | `electric-arc`     | EAF (scrap-based, grid-powered)   | 0.3 – 0.7                |
//! | `direct-reduction` | DRI-EAF (natural gas or green H₂) | 0.1 – 1.4                |
//!
//! Source: worldsteel CO₂ data collection, IEA Iron & Steel Tracker 2023.
//! These are **reference ranges**, not mandated compliance thresholds.
//!
//! ## Schema fields (steel v1.0.0)
//! - `productionRoute`        — enum: `blast-furnace | electric-arc | direct-reduction`
//! - `recycledScrapContentPct`— 0–100 % (scrap share of total charge)
//! - `co2ePerTonneSteel`      — tCO₂e per tonne of steel, non-negative
//! - `productCategory`        — enum: `flat | long | tube | specialty | other`
//! - `countryOfProduction`    — ISO 3166-1 alpha-2

// ── Production route CO₂e reference ranges (tCO₂e / tonne steel) ─────────────
// worldsteel / IEA benchmark values — NOT finalized EU DPP thresholds.
// Implement compliance-checking functions here once a regulatory threshold is
// published in an ESPR delegated act.

/// Typical upper bound for BF-BOF CO₂e intensity (tCO₂e / t).
pub const CO2E_REF_MAX_BLAST_FURNACE_T_PER_T: f64 = 2.5;

/// Typical upper bound for EAF CO₂e intensity (tCO₂e / t).
pub const CO2E_REF_MAX_ELECTRIC_ARC_T_PER_T: f64 = 0.7;

/// Typical upper bound for DRI-EAF CO₂e intensity (tCO₂e / t).
pub const CO2E_REF_MAX_DIRECT_REDUCTION_T_PER_T: f64 = 1.4;

// Placeholder — compliance-checking functions to be added once an ESPR
// delegated act specifies mandatory CO₂e thresholds for steel DPP.
