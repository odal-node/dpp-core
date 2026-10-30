//! RFC 8785 canonicalisation — the byte form every signature is computed over
//! and every content-binding verifier recomputes. The `crypto` target times
//! signing as a whole; this isolates the part that grows with the document.

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};
use dpp_crypto::jws::canonical::canonicalize;
use serde_json::json;

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::battery_passport_with_materials;

fn canonical_benchmarks(c: &mut Criterion) {
    // The payload the `crypto` target signs, so the two reports can be read side
    // by side.
    let small = json!({"passport_id": "bench-001", "status": "draft"});
    c.bench_function("jcs_small_payload", |b| {
        b.iter(|| canonicalize(&small).unwrap());
    });

    // A whole passport, as it is signed. Throughput is the canonical size.
    let mut group = c.benchmark_group("jcs_passport");
    for materials in [1_usize, 500] {
        let value = serde_json::to_value(battery_passport_with_materials(materials)).unwrap();
        let size = canonicalize(&value).unwrap().len();
        group.throughput(Throughput::Bytes(size as u64));
        group.bench_with_input(
            BenchmarkId::from_parameter(materials),
            &value,
            |b, value| {
                b.iter(|| canonicalize(value).unwrap());
            },
        );
    }
    group.finish();
}

criterion_group!(benches, canonical_benchmarks);
criterion_main!(benches);
