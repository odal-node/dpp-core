//! Input parameters for the cradle-to-gate CO₂e calculator.

use serde::{Deserialize, Serialize};

/// One material line of the bill of materials, with its embodied-emissions factor.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MaterialFootprint {
    /// Mass of this material in the finished product, in kilograms.
    pub mass_kg: f64,
    /// Embodied emissions per kilogram of this material, in kg CO₂e/kg
    /// (e.g. from the EU JRC LCA database or ecoinvent).
    pub emission_factor_kg_co2e_per_kg: f64,
}

/// Inputs for a single product's cradle-to-gate footprint.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Co2eInputs {
    /// Bill of materials with per-material emission factors.
    pub materials: Vec<MaterialFootprint>,
    /// Manufacturing energy consumed per unit, in kWh.
    pub energy_kwh: f64,
    /// Emissions per kWh of the production grid, in kg CO₂e/kWh.
    pub grid_factor_kg_co2e_per_kwh: f64,
}
