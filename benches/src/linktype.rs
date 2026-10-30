//! Digital Link content negotiation: picking which representation of a passport
//! to serve, and reading the `Accept` header that asks for it. This is the
//! per-request work of a resolver, ahead of any lookup.

use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dpp_digital_link::{DppMediaType, Gs1LinkType, LinkDescriptor, ResolutionRequest, negotiate};
use dpp_domain::{Audience, Disclosure};

const LINK_TYPES: [Gs1LinkType; 6] = [
    Gs1LinkType::ProductInformationPage,
    Gs1LinkType::SustainabilityInfo,
    Gs1LinkType::RecyclingInfo,
    Gs1LinkType::CertificationInfo,
    Gs1LinkType::Instructions,
    Gs1LinkType::Traceability,
];
const MEDIA_TYPES: [DppMediaType; 4] = [
    DppMediaType::Json,
    DppMediaType::JsonLd,
    DppMediaType::Html,
    DppMediaType::Pdf,
];
const DISCLOSURES: [Disclosure; 4] = [
    Disclosure::Public,
    Disclosure::Restricted,
    Disclosure::Conformity,
    Disclosure::Individual,
];

/// `n` descriptors cycling through link types, media types and disclosure
/// classes. The disclosure class steps every second descriptor, so it is not
/// locked to the media type, which steps every one.
fn linkset(n: usize) -> Vec<LinkDescriptor> {
    (0..n)
        .map(|i| LinkDescriptor {
            href: format!("https://passports.operator.example/dpp/{i}"),
            link_type: LINK_TYPES[i % LINK_TYPES.len()].clone(),
            media_type: MEDIA_TYPES[i % MEDIA_TYPES.len()].clone(),
            disclosure: DISCLOSURES[(i / 2) % DISCLOSURES.len()],
            title: Some(format!("Representation {i}")),
            language: Some("en".into()),
        })
        .collect()
}

fn negotiation(c: &mut Criterion) {
    let mut group = c.benchmark_group("linktype_negotiate");
    for n in [5_usize, 50] {
        let links = linkset(n);

        // No hints: the first representation the audience may see.
        let any = ResolutionRequest {
            link_type: None,
            media_type: None,
            audience: None,
        };
        group.bench_with_input(BenchmarkId::new("any", n), &links, |b, links| {
            b.iter(|| negotiate(links, &any));
        });

        // A link type and a media type, as a client following a linkset would ask.
        // Both are present in the smallest set, so this is a hit at every size.
        let typed = ResolutionRequest {
            link_type: Some(Gs1LinkType::SustainabilityInfo),
            media_type: Some(DppMediaType::JsonLd),
            audience: Some(Audience::Authority),
        };
        group.bench_with_input(
            BenchmarkId::new("typed_authority", n),
            &links,
            |b, links| {
                b.iter(|| negotiate(links, &typed));
            },
        );

        // A link type the set does not carry: the not-found path.
        let absent = ResolutionRequest {
            link_type: Some(Gs1LinkType::Successor),
            media_type: None,
            audience: None,
        };

        // Each case has to take the path its name says, at every size.
        assert!(negotiate(&links, &any).is_some(), "any/{n} should be a hit");
        assert!(
            negotiate(&links, &typed).is_some(),
            "typed/{n} should be a hit"
        );
        assert!(
            negotiate(&links, &absent).is_none(),
            "no_match/{n} should miss"
        );
        group.bench_with_input(BenchmarkId::new("no_match", n), &links, |b, links| {
            b.iter(|| negotiate(links, &absent));
        });
    }
    group.finish();
}

fn accept_header(c: &mut Criterion) {
    let mut group = c.benchmark_group("accept_header");
    for (label, header) in [
        ("single", "application/json"),
        (
            "browser",
            "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8",
        ),
        (
            "weighted_client",
            "application/ld+json;q=0.9, application/json;q=0.8, text/html;q=0.7, */*;q=0.1",
        ),
    ] {
        group.bench_function(label, |b| {
            b.iter(|| ResolutionRequest::from_accept_header(header))
        });
    }
    group.finish();
}

criterion_group!(benches, negotiation, accept_header);
criterion_main!(benches);
