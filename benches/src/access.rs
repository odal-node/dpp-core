//! Audience redaction — `redact_passport`, the one filter every served view of a
//! passport goes through. It runs on each public and credentialed read, so its
//! cost is paid per request rather than per publish.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dpp_domain::Audience;
use dpp_domain::access::redact_passport;

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::{battery_passport, battery_passport_with_materials};

const AUDIENCES: [(&str, Audience); 3] = [
    ("public", Audience::Public),
    ("legitimate_interest", Audience::LegitimateInterest),
    ("authority", Audience::Authority),
];

/// Size of the served view, to check what a benchmark is about to time.
fn view_len(passport: &dpp_domain::Passport, audience: Audience) -> usize {
    redact_passport(passport, audience)
        .into_value()
        .to_string()
        .len()
}

fn access_benchmarks(c: &mut Criterion) {
    let passport = battery_passport();

    // A benchmark of the wrong path is worse than none. The fixture has to
    // resolve a policy, so the filtered path runs, and the public view has to be
    // the narrowest; if either stops being true the figures below would still
    // print, and mean something else. The other two audiences are not ordered:
    // legitimate interest sees the individual class and an authority the
    // conformity class, so neither view contains the other.
    let sizes: Vec<usize> = AUDIENCES
        .iter()
        .map(|(_, audience)| view_len(&passport, *audience))
        .collect();
    assert!(
        sizes[0] < sizes[1] && sizes[0] < sizes[2],
        "the public view should be smaller than the other two: {sizes:?}"
    );

    // The public view keeps the fewest fields, so it is the cheapest to build.
    let mut by_audience = c.benchmark_group("redact_by_audience");
    for (label, audience) in AUDIENCES {
        by_audience.bench_function(label, |b| {
            b.iter(|| redact_passport(&passport, audience));
        });
    }
    by_audience.finish();

    // A schema version this build does not carry resolves to no policy, and the
    // product group data is reduced to its tag. That is the fail-closed path, and
    // it is cheaper than the filtered one; a regression here means the fallback
    // started doing real work.
    let mut unknown_version = battery_passport();
    unknown_version.schema_version = "0.0.0".into();
    assert!(
        view_len(&unknown_version, Audience::Public) < sizes[0],
        "the fail-closed view should be smaller than the filtered public one"
    );
    c.bench_function("redact_fail_closed_unknown_schema_version", |b| {
        b.iter(|| redact_passport(&unknown_version, Audience::Public));
    });

    // Redaction serialises the whole passport before it filters, so it grows with
    // the record. The bill of materials is the repeating part of the envelope.
    let mut by_materials = c.benchmark_group("redact_public_by_materials");
    for materials in [1_usize, 50, 500] {
        let passport = battery_passport_with_materials(materials);
        by_materials.bench_with_input(
            BenchmarkId::from_parameter(materials),
            &passport,
            |b, passport| {
                b.iter(|| redact_passport(passport, Audience::Public));
            },
        );
    }
    by_materials.finish();
}

criterion_group!(benches, access_benchmarks);
criterion_main!(benches);
