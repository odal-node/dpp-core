//! Benchmark the AAS (Asset Administration Shell) mapping path —
//! `build_aas_from_passport` — for registry / data-space interoperability.
//! This runs on every passport export to a Catena-X / IDTA consumer.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dpp_aas::{AssetIdentity, build_aas_from_passport};
use dpp_domain::{Audience, Passport};

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::{battery_passport, battery_passport_with_materials};

/// The identity the projection is asked for. Benchmarked on the real path: the
/// shell names the asset after the scheme that issued its identifier.
fn identity_of(p: &Passport) -> AssetIdentity<'_> {
    AssetIdentity::from_passport(p).expect("the benchmark fixture carries an identifier")
}

/// AAS submodel mapping and the build-and-serialise path used for export, per
/// audience and across bills of materials.
fn aas_benchmarks(c: &mut Criterion) {
    let passport = battery_passport();

    c.bench_function("aas_build_from_battery_passport", |b| {
        b.iter(|| {
            build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
                .expect("masking")
        });
    });

    c.bench_function("aas_build_and_serialise", |b| {
        b.iter(|| {
            let (shell, submodels) =
                build_aas_from_passport(&passport, identity_of(&passport), Audience::Public)
                    .expect("masking");
            (
                serde_json::to_string(&shell).unwrap(),
                serde_json::to_string(&submodels).unwrap(),
            )
        });
    });

    // The projection masks per audience, and the output differs: public keeps
    // the fewest fields, and the other two audiences see different classes
    // rather than more of the same. Whether that shows in the time is what this
    // group is for.
    let projected_len = |audience: Audience| {
        let (_, submodels) =
            build_aas_from_passport(&passport, identity_of(&passport), audience).expect("masking");
        serde_json::to_string(&submodels).unwrap().len()
    };
    let public_len = projected_len(Audience::Public);
    assert!(
        public_len < projected_len(Audience::LegitimateInterest)
            && public_len < projected_len(Audience::Authority),
        "the public projection should carry less than either credentialed one"
    );
    let mut by_audience = c.benchmark_group("aas_build_by_audience");
    for (label, audience) in [
        ("public", Audience::Public),
        ("legitimate_interest", Audience::LegitimateInterest),
        ("authority", Audience::Authority),
    ] {
        by_audience.bench_function(label, |b| {
            b.iter(|| {
                build_aas_from_passport(&passport, identity_of(&passport), audience)
                    .expect("masking")
            });
        });
    }
    by_audience.finish();

    // How the projection grows with the bill of materials, which is the one
    // repeating part of the envelope the materials submodel maps line by line.
    let mut by_materials = c.benchmark_group("aas_build_by_materials");
    for materials in [1_usize, 50, 500] {
        let passport = battery_passport_with_materials(materials);
        by_materials.bench_with_input(
            BenchmarkId::from_parameter(materials),
            &passport,
            |b, passport| {
                b.iter(|| {
                    build_aas_from_passport(passport, identity_of(passport), Audience::Public)
                        .expect("masking")
                });
            },
        );
    }
    by_materials.finish();
}

criterion_group!(benches, aas_benchmarks);
criterion_main!(benches);
