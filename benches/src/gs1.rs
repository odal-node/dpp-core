use criterion::{Criterion, criterion_group, criterion_main};
use dpp_digital_link::{DigitalLink, build_qr_url};

#[path = "fixtures.rs"]
mod fixtures;
use fixtures::battery_passport;

/// Parsing a Digital Link that carries a serial, and one that carries a batch and
/// a serial.
fn gs1_benchmarks(c: &mut Criterion) {
    c.bench_function("gs1_parse_with_serial", |b| {
        b.iter(|| DigitalLink::parse("https://example.com/01/09506000134352/21/ABC123").unwrap());
    });

    c.bench_function("gs1_parse_full", |b| {
        b.iter(|| {
            DigitalLink::parse("https://example.com/01/09506000134352/10/BATCH01/21/SN001").unwrap()
        });
    });
}

/// The other shapes a resolver is handed: the bare key, a query string, a path
/// prefix, an escaped value, and the two refusals.
fn gs1_forms(c: &mut Criterion) {
    let accepted = [
        ("gtin_only", "https://example.com/01/09506000134352"),
        (
            "with_query",
            "https://example.com/01/09506000134352?linkType=gs1:pip",
        ),
        (
            "path_prefix",
            "https://example.com/some/path/prefix/01/09506000134352/10/BATCH01/21/SN001",
        ),
        (
            "escaped_serial",
            "https://example.com/01/09506000134352/21/AB%2FC",
        ),
    ];
    let refused = [
        ("bad_check_digit", "https://example.com/01/09506000134353"),
        ("bad_scheme", "ftp://example.com/01/09506000134352"),
    ];

    // A refusal is a fast path and a different one; make sure each case is the
    // kind its group says it is.
    for (label, uri) in accepted {
        assert!(DigitalLink::parse(uri).is_ok(), "{label} should parse");
    }
    for (label, uri) in refused {
        assert!(
            DigitalLink::parse(uri).is_err(),
            "{label} should be refused"
        );
    }

    let mut parse = c.benchmark_group("gs1_parse_forms");
    for (label, uri) in accepted {
        parse.bench_function(label, |b| b.iter(|| DigitalLink::parse(uri).unwrap()));
    }
    parse.finish();

    let mut refuse = c.benchmark_group("gs1_parse_refused");
    for (label, uri) in refused {
        refuse.bench_function(label, |b| b.iter(|| DigitalLink::parse(uri).unwrap_err()));
    }
    refuse.finish();

    let link =
        DigitalLink::parse("https://example.com/01/09506000134352/10/BATCH01/21/SN001").unwrap();
    c.bench_function("gs1_build_full", |b| b.iter(|| link.build()));

    // Minting the carrier URL printed on a label, from a passport.
    let passport = battery_passport();
    assert!(
        matches!(build_qr_url("https://example.com", &passport), Ok(Some(_))),
        "the fixture should yield a carrier URL"
    );
    c.bench_function("gs1_build_qr_url_battery", |b| {
        b.iter(|| build_qr_url("https://example.com", &passport).unwrap());
    });
}

criterion_group!(benches, gs1_benchmarks, gs1_forms);
criterion_main!(benches);
