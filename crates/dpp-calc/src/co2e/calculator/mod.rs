//! The cradle-to-gate CO₂e calculation: bill of materials + energy → footprint.
//!
//! ## PEF lifecycle stages
//!
//! The EU PEF method covers the full product lifecycle (raw-material extraction,
//! production, distribution, use, end-of-life). This calculator covers the
//! **RawMaterials + Production** stages (cradle-to-gate), as declared in
//! [`Co2eResult::declared_stages`]. Future calculators will extend to remaining
//! stages without changing this module's contract.
//!
//! ## Methodology
//!
//! ```text
//! co2e_kg = Σ (material.mass_kg × material.emission_factor) + energy_kwh × grid_factor
//! ```
//!
//! Operator-supplied emission factors. The full PEF method (EU JRC LCA database,
//! country-specific grid factors, allocation rules) refines those factors and is
//! gated on a signed data license (`real-factors` feature).

mod calculation;
#[cfg(test)]
mod tests;

pub use calculation::*;
