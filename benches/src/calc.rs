//! Benchmarks for the dpp-calc compute kernels: cradle-to-gate CO₂e and the
//! simplified repairability heuristic. These are the hot paths invoked on every
//! passport publish that requests a calculated metric.

use criterion::{Criterion, criterion_group, criterion_main};
use dpp_calc::co2e::{self, Co2eInputs, CradleToGateRuleset, MaterialFootprint};
use dpp_calc::repairability::{
    self, SimplifiedRepairabilityHeuristic, parameters::RepairabilityInputs,
};

fn small_co2e_inputs() -> Co2eInputs {
    Co2eInputs {
        materials: vec![
            MaterialFootprint {
                mass_kg: 0.5,
                emission_factor_kg_co2e_per_kg: 8.0,
            },
            MaterialFootprint {
                mass_kg: 0.2,
                emission_factor_kg_co2e_per_kg: 3.0,
            },
        ],
        energy_kwh: 1.5,
        grid_factor_kg_co2e_per_kwh: 0.4,
    }
}

fn bill_of_materials(n: usize) -> Co2eInputs {
    Co2eInputs {
        materials: (0..n)
            .map(|i| MaterialFootprint {
                mass_kg: 0.1 + (i as f64) * 0.01,
                emission_factor_kg_co2e_per_kg: 2.0 + (i % 7) as f64,
            })
            .collect(),
        energy_kwh: 12.0,
        grid_factor_kg_co2e_per_kwh: 0.35,
    }
}

fn repairability_inputs() -> RepairabilityInputs {
    RepairabilityInputs {
        disassembly: 2,
        spare_parts: 2,
        repair_info: 1,
        diagnostic_tools: 1,
        software_updatability: 2,
        customer_support: 1,
    }
}

fn calc_benchmarks(c: &mut Criterion) {
    let small = small_co2e_inputs();
    c.bench_function("co2e_cradle_to_gate_small", |b| {
        b.iter(|| co2e::calculate(&small, &CradleToGateRuleset).unwrap());
    });

    let big = bill_of_materials(50);
    c.bench_function("co2e_cradle_to_gate_50_materials", |b| {
        b.iter(|| co2e::calculate(&big, &CradleToGateRuleset).unwrap());
    });

    let rep = repairability_inputs();
    c.bench_function("repairability_heuristic_smartphone", |b| {
        b.iter(|| repairability::calculate(&rep, &SimplifiedRepairabilityHeuristic).unwrap());
    });
}

criterion_group!(benches, calc_benchmarks);
criterion_main!(benches);
