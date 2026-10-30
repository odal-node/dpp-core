//! `Passport` serialisation. A passport is stored and served as a JSON document,
//! so these four conversions are paid on every read and every write, ahead of
//! anything the domain does with the record.

use criterion::{BatchSize, BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dpp_domain::Passport;

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::battery_passport_with_materials;

fn serde_benchmarks(c: &mut Criterion) {
    // One line is the record as it is usually found; 500 is a deep bill of
    // materials. The throughput is the document's size in bytes, so the report
    // shows MB/s and a change in size is not mistaken for a change in speed.
    for materials in [1_usize, 500] {
        let passport = battery_passport_with_materials(materials);
        let value = serde_json::to_value(&passport).unwrap();
        let text = serde_json::to_string(&passport).unwrap();

        let mut group = c.benchmark_group("passport_serde");
        group.throughput(Throughput::Bytes(text.len() as u64));

        group.bench_with_input(
            BenchmarkId::new("to_value", materials),
            &passport,
            |b, passport| b.iter(|| serde_json::to_value(passport).unwrap()),
        );
        group.bench_with_input(
            BenchmarkId::new("to_string", materials),
            &passport,
            |b, passport| b.iter(|| serde_json::to_string(passport).unwrap()),
        );
        group.bench_with_input(BenchmarkId::new("from_str", materials), &text, |b, text| {
            b.iter(|| serde_json::from_str::<Passport>(text).unwrap())
        });
        // `from_value` consumes its input, so the copy it needs is made in the
        // untimed setup rather than inside the measurement.
        group.bench_with_input(
            BenchmarkId::new("from_value", materials),
            &value,
            |b, value| {
                b.iter_batched(
                    || value.clone(),
                    |value| serde_json::from_value::<Passport>(value).unwrap(),
                    BatchSize::SmallInput,
                )
            },
        );
        group.finish();
    }
}

criterion_group!(benches, serde_benchmarks);
criterion_main!(benches);
