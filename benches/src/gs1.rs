use criterion::{Criterion, criterion_group, criterion_main};
use dpp_digital_link::DigitalLink;

fn gs1_benchmarks(c: &mut Criterion) {
    c.bench_function("gs1_parse_with_serial", |b| {
        b.iter(|| {
            DigitalLink::parse("https://id.odal-node.io/01/09506000134352/21/ABC123").unwrap()
        });
    });

    c.bench_function("gs1_parse_full", |b| {
        b.iter(|| {
            DigitalLink::parse("https://id.odal-node.io/01/09506000134352/10/BATCH01/21/SN001")
                .unwrap()
        });
    });
}

criterion_group!(benches, gs1_benchmarks);
criterion_main!(benches);
