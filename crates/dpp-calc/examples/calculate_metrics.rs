//! Compute a product's CO₂e footprint and simplified repairability heuristic.
//!
//! Run with: `cargo run --example calculate_metrics -p dpp-calc`

use dpp_calc::co2e::{self, Co2eInputs, CradleToGateRuleset, MaterialFootprint};
use dpp_calc::repairability::{
    self, SimplifiedRepairabilityHeuristic, parameters::RepairabilityInputs,
};

fn main() {
    // Cradle-to-gate CO₂e for a small battery (materials + manufacturing energy).
    let footprint = co2e::calculate(
        &Co2eInputs {
            materials: vec![
                MaterialFootprint {
                    mass_kg: 0.5,
                    emission_factor_kg_co2e_per_kg: 8.0, // e.g. recycled aluminium
                },
                MaterialFootprint {
                    mass_kg: 0.2,
                    emission_factor_kg_co2e_per_kg: 3.0,
                },
            ],
            energy_kwh: 1.5,
            grid_factor_kg_co2e_per_kwh: 0.4,
        },
        &CradleToGateRuleset,
    )
    .expect("valid inputs");

    println!(
        "CO₂e: {:.2} kg  (materials {:.2} + energy {:.2})  stages={:?}  receipt={}",
        footprint.total_co2e_kg,
        footprint.material_co2e_kg,
        footprint.energy_co2e_kg,
        footprint.declared_stages,
        footprint.receipt.receipt_id,
    );
    for (i, line) in footprint.material_breakdown.iter().enumerate() {
        println!(
            "  material[{i}]: {:.3} kg × {:.3} kg CO₂e/kg = {:.3} kg CO₂e",
            line.mass_kg, line.emission_factor_kg_co2e_per_kg, line.co2e_kg
        );
    }

    // Simplified repairability heuristic band (A–E) for a smartphone — not the
    // EU 2023/1669 regulatory class.
    let rep = repairability::calculate(
        &RepairabilityInputs {
            disassembly: 2,
            spare_parts: 2,
            repair_info: 1,
            diagnostic_tools: 1,
            software_updatability: 2,
            customer_support: 1,
        },
        &SimplifiedRepairabilityHeuristic,
    )
    .expect("valid inputs");

    println!(
        "Repairability: {}  ({:.2}/10)  ruleset={}@{}",
        rep.class, rep.numeric_score, rep.receipt.ruleset_id, rep.receipt.ruleset_version,
    );
    let c = &rep.contributions;
    println!(
        "  disassembly={:.2} spare_parts={:.2} repair_info={:.2} \
         diagnostic={:.2} software={:.2} support={:.2}",
        c.disassembly,
        c.spare_parts,
        c.repair_info,
        c.diagnostic_tools,
        c.software_updatability,
        c.customer_support,
    );
}
