//! Benchmarks for the dpp-calc compute kernels: cradle-to-gate CO₂e and the
//! simplified repairability heuristic.
//!
//! These are **not** on any hot path — nothing in either repository invokes
//! them. This file previously described them as "the hot paths invoked on every
//! passport publish that requests a calculated metric", which would have told a
//! reader looking for the wiring that it existed. The benchmarks are still worth
//! keeping: they bound the cost of a bill-of-materials walk before a caller
//! commits to one on a publish path.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dpp_calc::co2e::{self, Co2eInputs, CradleToGateRuleset, MaterialFootprint};
use dpp_calc::repairability::{
    self, SimplifiedRepairabilityHeuristic, parameters::RepairabilityInputs,
};

/// The small bill of materials the CO2e benches start from.
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

/// A bill of `n` materials whose masses and emission factors vary, so no two are
/// alike.
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

/// A fixed set of repairability scores.
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

/// Cradle-to-gate CO2e from a small bill and up a ladder of materials, and the
/// simplified repairability heuristic.
fn calc_benchmarks(c: &mut Criterion) {
    let clock = dpp_calc::clock::AssessmentClock::placed_on(
        chrono::NaiveDate::from_ymd_opt(2026, 1, 1).expect("valid date"),
    );

    let small = small_co2e_inputs();
    c.bench_function("co2e_cradle_to_gate_small", |b| {
        b.iter(|| co2e::calculate(&small, &CradleToGateRuleset, clock).unwrap());
    });

    let big = bill_of_materials(50);
    c.bench_function("co2e_cradle_to_gate_50_materials", |b| {
        b.iter(|| co2e::calculate(&big, &CradleToGateRuleset, clock).unwrap());
    });

    // The two points above, extended into a ladder. Throughput is materials per
    // second, so a change in how the cost grows shows as a change in slope.
    let mut ladder = c.benchmark_group("co2e_cradle_to_gate_by_materials");
    for materials in [10_usize, 100, 1_000, 10_000] {
        let inputs = bill_of_materials(materials);
        ladder.throughput(Throughput::Elements(materials as u64));
        ladder.bench_with_input(
            BenchmarkId::from_parameter(materials),
            &inputs,
            |b, inputs| b.iter(|| co2e::calculate(inputs, &CradleToGateRuleset, clock).unwrap()),
        );
    }
    ladder.finish();

    let rep = repairability_inputs();
    c.bench_function("repairability_heuristic_smartphone", |b| {
        b.iter(|| {
            repairability::calculate(&rep, &SimplifiedRepairabilityHeuristic, clock).unwrap()
        });
    });
}

criterion_group!(benches, calc_benchmarks);
criterion_main!(benches);
