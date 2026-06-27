//! Validation helpers for sector payload lists.
//!
//! The cross-field rule logic lives once in `dpp-rules` (shared with the Wasm
//! sector plugins). These thin adapters map the domain structs onto that crate's
//! primitive inputs, preserving the public API and error messages.

use crate::domain::sector::data::{FibreEntry, SurfactantEntry, SvhcSubstance};

/// Validate a list of SVHC substance declarations. Delegates to [`dpp_rules`].
pub fn validate_svhc_substances(substances: &[SvhcSubstance]) -> Result<(), String> {
    let inputs: Vec<dpp_rules::SvhcInput<'_>> = substances
        .iter()
        .map(|s| dpp_rules::SvhcInput {
            cas_number: &s.cas_number,
            substance_name: &s.substance_name,
            concentration_pct: s.concentration_pct,
        })
        .collect();
    dpp_rules::validate_svhc_substances(&inputs)
}

/// Validate a textile fibre composition list. Delegates to [`dpp_rules`].
pub fn validate_fibre_composition(fibres: &[FibreEntry]) -> Result<(), String> {
    let inputs: Vec<dpp_rules::FibreInput<'_>> = fibres
        .iter()
        .map(|f| dpp_rules::FibreInput {
            fibre: &f.fibre,
            pct: f.pct,
            country_of_origin: f.country_of_origin.as_deref(),
        })
        .collect();
    dpp_rules::validate_fibre_composition(&inputs)
}

/// Validate a battery's operating temperature range (`min < max` when both are
/// declared). Delegates to [`dpp_rules`].
pub fn validate_battery_operating_temp(
    min_c: Option<f64>,
    max_c: Option<f64>,
) -> Result<(), String> {
    dpp_rules::batteries::chemistry::validate_operating_temp_range(min_c, max_c)
}

/// Metals declared with recycled content `> 0` that the battery chemistry does
/// not contain (a data-integrity contradiction, e.g. cobalt on LFP). Delegates
/// to [`dpp_rules`]. `chemistry` is the serde code (`"LFP"`, `"NMC"`, …).
pub fn battery_recycled_chemistry_conflicts(
    chemistry: &str,
    cobalt_pct: Option<f64>,
    lithium_pct: Option<f64>,
    nickel_pct: Option<f64>,
    lead_pct: Option<f64>,
) -> Vec<&'static str> {
    dpp_rules::batteries::recycled_content::recycled_content_chemistry_conflicts(
        chemistry,
        cobalt_pct,
        lithium_pct,
        nickel_pct,
        lead_pct,
    )
}

/// Validate a detergent surfactant list. Delegates to [`dpp_rules`].
pub fn validate_surfactants(surfactants: &[SurfactantEntry]) -> Result<(), String> {
    let inputs: Vec<dpp_rules::SurfactantInput<'_>> = surfactants
        .iter()
        .map(|s| dpp_rules::SurfactantInput {
            name: &s.name,
            concentration_band: &s.concentration_band,
            biodegradable: s.biodegradable,
            cas_number: s.cas_number.as_deref(),
        })
        .collect();
    dpp_rules::validate_surfactants(&inputs)
}
