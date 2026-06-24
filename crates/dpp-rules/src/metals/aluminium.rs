//! Aluminium cross-field rules — EU ESPR + CBAM (EU Regulation 2023/956).
//!
//! ## Regulatory status
//! The aluminium DPP mandate is expected around 2030. CBAM (EU 2023/956) covers
//! embedded-carbon reporting for aluminium imports but does **not** set
//! production-level CO₂e thresholds that would make a product non-compliant for
//! DPP purposes. The thresholds below are **industry/CBAM benchmark values**
//! used by the `sector-aluminium` plugin; they are not finalized EU DPP mandates.
//!
//! ## Production routes and CO₂e reference thresholds (kg CO₂e / tonne Al)
//! | Route                | Threshold used by plugin |
//! |----------------------|--------------------------|
//! | `primary`            | ≤ 10 000                 |
//! | `secondary-recycled` | ≤ 1 000                  |
//! | `mixed`              | ≤ 5 000                  |
//!
//! Source: CBAM benchmark values / `sector-aluminium` plugin. These are
//! informational reference points until a finalized EU DPP threshold is adopted.
//!
//! ## Schema fields (aluminium v1.0.0)
//! - `alloyGrade`         — free-form string (e.g. `"1xxx"`, `"6061"`)
//! - `productionRoute`    — enum: `primary | secondary-recycled | mixed`
//! - `co2ePerTonneKg`     — kg CO₂e per tonne, non-negative
//! - `recycledContentPct` — 0–100 %
//! - `countryOfProduction`— ISO 3166-1 alpha-2

// ── Production route CO₂e reference thresholds ───────────────────────────────
// These values are CBAM benchmarks used by the sector-aluminium plugin.
// They are NOT finalized EU DPP compliance thresholds.

/// Reference CO₂e threshold for primary (Hall-Héroult) aluminium (kg CO₂e / tonne).
pub const CO2E_REF_PRIMARY_KG_PER_T: f64 = 10_000.0;

/// Reference CO₂e threshold for secondary-recycled aluminium (kg CO₂e / tonne).
pub const CO2E_REF_SECONDARY_RECYCLED_KG_PER_T: f64 = 1_000.0;

/// Reference CO₂e threshold for mixed-route aluminium (kg CO₂e / tonne).
pub const CO2E_REF_MIXED_KG_PER_T: f64 = 5_000.0;

/// Whether a declared CO₂e intensity is within the reference threshold for
/// the given production route.
///
/// Returns `false` for any unrecognised route string (caller should validate
/// the route enum separately via JSON Schema).
///
/// **Note:** this is a benchmark check, not an EU DPP compliance determination.
/// The aluminium sector plugin calls this function; it returns `NOT_ASSESSED`
/// until a finalized regulatory threshold is published.
#[must_use]
pub fn co2e_within_route_threshold(route: &str, co2e_kg_per_tonne: f64) -> bool {
    let threshold = match route {
        "primary" => CO2E_REF_PRIMARY_KG_PER_T,
        "secondary-recycled" => CO2E_REF_SECONDARY_RECYCLED_KG_PER_T,
        "mixed" => CO2E_REF_MIXED_KG_PER_T,
        _ => return false,
    };
    co2e_kg_per_tonne <= threshold
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn primary_at_threshold_passes() {
        assert!(co2e_within_route_threshold("primary", 10_000.0));
        assert!(co2e_within_route_threshold("primary", 6_000.0));
    }

    #[test]
    fn primary_above_threshold_fails() {
        assert!(!co2e_within_route_threshold("primary", 10_001.0));
    }

    #[test]
    fn secondary_recycled_threshold() {
        assert!(co2e_within_route_threshold("secondary-recycled", 600.0));
        assert!(!co2e_within_route_threshold("secondary-recycled", 1_001.0));
    }

    #[test]
    fn mixed_threshold() {
        assert!(co2e_within_route_threshold("mixed", 5_000.0));
        assert!(!co2e_within_route_threshold("mixed", 5_001.0));
    }

    #[test]
    fn unknown_route_fails() {
        assert!(!co2e_within_route_threshold("unknown", 0.0));
    }
}
