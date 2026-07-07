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

use serde::{Deserialize, Serialize};

use super::parameters::Co2eInputs;
use super::thresholds::Co2eRuleset;
use crate::error::CalcError;
use crate::receipt::{CalculationReceipt, input_hash, jcs_hash};

/// A single stage in the EU PEF product lifecycle.
///
/// Use [`Co2eResult::declared_stages`] to understand which stages a result covers.
/// The cradle-to-gate [`calculate`] covers `RawMaterials` + `Production`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleStage {
    /// Extraction and processing of raw materials (cradle).
    RawMaterials,
    /// Manufacturing, assembly, and finishing (gate).
    Production,
    /// Packaging, transport, and distribution to point of sale.
    Distribution,
    /// Energy and resource consumption during product use.
    Use,
    /// Waste processing, recycling, and disposal.
    EndOfLife,
}

/// CO₂e contribution for one material line — the audit breakdown that backs the total.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaterialLineResult {
    /// Mass of this material, in kg (echoed from input for self-documenting receipts).
    pub mass_kg: f64,
    /// Emission factor used, in kg CO₂e/kg.
    pub emission_factor_kg_co2e_per_kg: f64,
    /// Embodied emissions for this line: `mass_kg × emission_factor`, in kg CO₂e.
    pub co2e_kg: f64,
}

/// Transparent breakdown of the footprint plus a proof-of-calculation receipt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Co2eResult {
    /// Embodied emissions from materials, kg CO₂e.
    pub material_co2e_kg: f64,
    /// Emissions from manufacturing energy, kg CO₂e.
    pub energy_co2e_kg: f64,
    /// Total cradle-to-gate footprint, kg CO₂e.
    pub total_co2e_kg: f64,
    /// Per-material breakdown — each entry corresponds to `Co2eInputs::materials[i]`.
    /// Enables a notified body to trace any line item without re-running the calculation.
    pub material_breakdown: Vec<MaterialLineResult>,
    /// Lifecycle stages this result covers (its declared PEF system boundary).
    pub declared_stages: Vec<LifecycleStage>,
    /// Proof-of-calculation receipt (input hash + output hash + ruleset citation).
    pub receipt: CalculationReceipt,
}

/// Calculate the cradle-to-gate CO₂e footprint for one unit.
///
/// Returns `Err(CalcError::InvalidInput)` for negative or non-finite inputs —
/// silent clamping is not appropriate for a legally cited compliance figure.
pub fn calculate(inputs: &Co2eInputs, ruleset: &dyn Co2eRuleset) -> Result<Co2eResult, CalcError> {
    validate_inputs(inputs)?;

    let material_breakdown: Vec<MaterialLineResult> = inputs
        .materials
        .iter()
        .map(|m| MaterialLineResult {
            mass_kg: m.mass_kg,
            emission_factor_kg_co2e_per_kg: m.emission_factor_kg_co2e_per_kg,
            co2e_kg: m.mass_kg * m.emission_factor_kg_co2e_per_kg,
        })
        .collect();

    let material_co2e_kg: f64 = material_breakdown.iter().map(|l| l.co2e_kg).sum();
    let energy_co2e_kg = inputs.energy_kwh * inputs.grid_factor_kg_co2e_per_kwh;
    let total_co2e_kg = material_co2e_kg + energy_co2e_kg;

    // Hash outputs before building the result (avoids chicken-and-egg with receipt).
    let output_hash = jcs_hash(&(total_co2e_kg, material_co2e_kg, energy_co2e_kg))?;

    let receipt = CalculationReceipt::new(
        input_hash(inputs)?,
        ruleset.id().0.as_str(),
        ruleset.version().0.as_str(),
    )
    .with_output_hash(output_hash);

    Ok(Co2eResult {
        material_co2e_kg,
        energy_co2e_kg,
        total_co2e_kg,
        material_breakdown,
        declared_stages: ruleset.declared_stages().to_vec(),
        receipt,
    })
}

fn validate_inputs(inputs: &Co2eInputs) -> Result<(), CalcError> {
    if !inputs.energy_kwh.is_finite() || inputs.energy_kwh < 0.0 {
        return Err(CalcError::InvalidInput(format!(
            "energy_kwh must be finite and ≥ 0; got {}",
            inputs.energy_kwh
        )));
    }
    if !inputs.grid_factor_kg_co2e_per_kwh.is_finite() || inputs.grid_factor_kg_co2e_per_kwh < 0.0 {
        return Err(CalcError::InvalidInput(format!(
            "grid_factor_kg_co2e_per_kwh must be finite and ≥ 0; got {}",
            inputs.grid_factor_kg_co2e_per_kwh
        )));
    }
    for (i, m) in inputs.materials.iter().enumerate() {
        if !m.mass_kg.is_finite() || m.mass_kg < 0.0 {
            return Err(CalcError::InvalidInput(format!(
                "materials[{i}].mass_kg must be finite and ≥ 0; got {}",
                m.mass_kg
            )));
        }
        if !m.emission_factor_kg_co2e_per_kg.is_finite() || m.emission_factor_kg_co2e_per_kg < 0.0 {
            return Err(CalcError::InvalidInput(format!(
                "materials[{i}].emission_factor must be finite and ≥ 0; got {}",
                m.emission_factor_kg_co2e_per_kg
            )));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::co2e::parameters::MaterialFootprint;
    use crate::co2e::thresholds::CradleToGateRuleset;

    fn material(mass: f64, factor: f64) -> MaterialFootprint {
        MaterialFootprint {
            mass_kg: mass,
            emission_factor_kg_co2e_per_kg: factor,
        }
    }

    #[test]
    fn rejects_negative_energy() {
        let inputs = Co2eInputs {
            materials: vec![material(1.0, 2.0)],
            energy_kwh: -1.0,
            grid_factor_kg_co2e_per_kwh: 0.4,
        };
        assert!(matches!(
            calculate(&inputs, &CradleToGateRuleset),
            Err(CalcError::InvalidInput(_))
        ));
    }

    #[test]
    fn rejects_non_finite_grid_factor() {
        let inputs = Co2eInputs {
            materials: vec![material(1.0, 2.0)],
            energy_kwh: 1.0,
            grid_factor_kg_co2e_per_kwh: f64::NAN,
        };
        assert!(matches!(
            calculate(&inputs, &CradleToGateRuleset),
            Err(CalcError::InvalidInput(_))
        ));
    }

    #[test]
    fn rejects_negative_material_mass() {
        let inputs = Co2eInputs {
            materials: vec![material(-0.5, 2.0)],
            energy_kwh: 1.0,
            grid_factor_kg_co2e_per_kwh: 0.4,
        };
        assert!(matches!(
            calculate(&inputs, &CradleToGateRuleset),
            Err(CalcError::InvalidInput(_))
        ));
    }

    #[test]
    fn rejects_negative_emission_factor() {
        let inputs = Co2eInputs {
            materials: vec![material(0.5, -2.0)],
            energy_kwh: 1.0,
            grid_factor_kg_co2e_per_kwh: 0.4,
        };
        assert!(matches!(
            calculate(&inputs, &CradleToGateRuleset),
            Err(CalcError::InvalidInput(_))
        ));
    }

    #[test]
    fn accepts_valid_inputs_and_sums_correctly() {
        let inputs = Co2eInputs {
            materials: vec![material(2.0, 3.0), material(1.0, 4.0)],
            energy_kwh: 10.0,
            grid_factor_kg_co2e_per_kwh: 0.5,
        };
        let result = calculate(&inputs, &CradleToGateRuleset).unwrap();
        // materials: 2*3 + 1*4 = 10; energy: 10*0.5 = 5; total = 15
        assert!((result.material_co2e_kg - 10.0).abs() < 1e-9);
        assert!((result.energy_co2e_kg - 5.0).abs() < 1e-9);
        assert!((result.total_co2e_kg - 15.0).abs() < 1e-9);
        assert_eq!(result.material_breakdown.len(), 2);
    }
}
