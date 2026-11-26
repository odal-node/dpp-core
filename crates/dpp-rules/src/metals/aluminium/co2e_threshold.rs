//! Aluminium CO2e reference values and the route threshold check.

// ── Production route CO₂e reference thresholds ───────────────────────────────
// These values are CBAM benchmarks used by the product group-aluminium plugin.
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
/// The aluminium product group plugin calls this function; it returns `NOT_ASSESSED`
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
