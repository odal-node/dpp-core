//! Link-type/media-type parsing, `Accept`-header q-values, and negotiation tests.

use super::*;

fn sample_descriptors() -> Vec<LinkDescriptor> {
    vec![
        LinkDescriptor {
            href: "https://id.example.com/dpp/12345/pip".into(),
            link_type: Gs1LinkType::ProductInformationPage,
            media_type: DppMediaType::Html,
            min_access_tier: AccessTier::Public,
            title: Some("Product Page".into()),
            language: Some("en".into()),
        },
        LinkDescriptor {
            href: "https://id.example.com/dpp/12345/data.json".into(),
            link_type: Gs1LinkType::DigitalProductPassport,
            media_type: DppMediaType::Json,
            min_access_tier: AccessTier::Public,
            title: Some("DPP Data (JSON)".into()),
            language: None,
        },
        LinkDescriptor {
            href: "https://id.example.com/dpp/12345/data.jsonld".into(),
            link_type: Gs1LinkType::DigitalProductPassport,
            media_type: DppMediaType::JsonLd,
            min_access_tier: AccessTier::Public,
            title: Some("DPP Data (JSON-LD)".into()),
            language: None,
        },
        LinkDescriptor {
            href: "https://id.example.com/dpp/12345/safety".into(),
            link_type: Gs1LinkType::SafetyInfo,
            media_type: DppMediaType::Json,
            min_access_tier: AccessTier::Professional,
            title: Some("SVHC Substances".into()),
            language: None,
        },
        LinkDescriptor {
            href: "https://id.example.com/dpp/12345/compliance".into(),
            link_type: Gs1LinkType::CertificationInfo,
            media_type: DppMediaType::Json,
            min_access_tier: AccessTier::Confidential,
            title: Some("Full Compliance Report".into()),
            language: None,
        },
    ]
}

#[test]
fn negotiate_default_returns_first_public() {
    let descs = sample_descriptors();
    let req = ResolutionRequest {
        link_type: None,
        media_type: None,
        access_tier: None,
    };
    let result = negotiate(&descs, &req).unwrap();
    assert_eq!(result.link_type, Gs1LinkType::ProductInformationPage);
}

#[test]
fn negotiate_by_link_type() {
    let descs = sample_descriptors();
    let req = ResolutionRequest {
        link_type: Some(Gs1LinkType::DigitalProductPassport),
        media_type: None,
        access_tier: None,
    };
    let result = negotiate(&descs, &req).unwrap();
    assert_eq!(result.link_type, Gs1LinkType::DigitalProductPassport);
}

#[test]
fn negotiate_by_link_type_and_media_type() {
    let descs = sample_descriptors();
    let req = ResolutionRequest {
        link_type: Some(Gs1LinkType::DigitalProductPassport),
        media_type: Some(DppMediaType::JsonLd),
        access_tier: None,
    };
    let result = negotiate(&descs, &req).unwrap();
    assert_eq!(result.media_type, DppMediaType::JsonLd);
}

#[test]
fn negotiate_filters_by_access_tier() {
    let descs = sample_descriptors();
    // Public caller cannot see Professional or Confidential resources
    let req = ResolutionRequest {
        link_type: Some(Gs1LinkType::SafetyInfo),
        media_type: None,
        access_tier: Some(AccessTier::Public),
    };
    let result = negotiate(&descs, &req);
    assert!(result.is_none(), "public caller should not see safety info");
}

#[test]
fn negotiate_professional_sees_safety_info() {
    let descs = sample_descriptors();
    let req = ResolutionRequest {
        link_type: Some(Gs1LinkType::SafetyInfo),
        media_type: None,
        access_tier: Some(AccessTier::Professional),
    };
    let result = negotiate(&descs, &req).unwrap();
    assert_eq!(result.link_type, Gs1LinkType::SafetyInfo);
}

#[test]
fn negotiate_confidential_sees_everything() {
    let descs = sample_descriptors();
    let req = ResolutionRequest {
        link_type: Some(Gs1LinkType::CertificationInfo),
        media_type: None,
        access_tier: Some(AccessTier::Confidential),
    };
    let result = negotiate(&descs, &req).unwrap();
    assert_eq!(result.link_type, Gs1LinkType::CertificationInfo);
}

#[test]
fn link_type_parse_shorthand() {
    assert_eq!(
        Gs1LinkType::parse("gs1:pip"),
        Gs1LinkType::ProductInformationPage
    );
    assert_eq!(
        Gs1LinkType::parse("gs1:dpp"),
        Gs1LinkType::DigitalProductPassport
    );
    assert_eq!(
        Gs1LinkType::parse("gs1:sustainabilityInfo"),
        Gs1LinkType::SustainabilityInfo
    );
}

#[test]
fn link_type_parse_full_uri() {
    assert_eq!(
        Gs1LinkType::parse("https://ref.gs1.org/voc/pip"),
        Gs1LinkType::ProductInformationPage
    );
}

/// GS1 Web Vocabulary namespace conformance: the canonical base is
/// `https://ref.gs1.org/voc/` (not `gs1.org/voc` or `www.gs1.org/voc`),
/// otherwise external GS1 link-type resolution breaks.
/// Source: GS1 Web Vocabulary (https://ref.gs1.org/voc/).
#[test]
fn gs1_uris_use_canonical_ref_namespace() {
    for lt in [
        Gs1LinkType::ProductInformationPage,
        Gs1LinkType::DigitalProductPassport,
        Gs1LinkType::SustainabilityInfo,
        Gs1LinkType::Traceability,
    ] {
        assert!(
            lt.as_gs1_uri().starts_with("https://ref.gs1.org/voc/"),
            "{lt:?} must use the canonical ref.gs1.org/voc namespace, got {}",
            lt.as_gs1_uri()
        );
    }
}

#[test]
fn link_type_parse_unknown() {
    let lt = Gs1LinkType::parse("https://example.com/custom");
    assert!(matches!(lt, Gs1LinkType::Custom(_)));
}

#[test]
fn link_type_round_trip_uri() {
    let lt = Gs1LinkType::DigitalProductPassport;
    let uri = lt.as_gs1_uri();
    let parsed = Gs1LinkType::parse(uri);
    assert_eq!(lt, parsed);
}

#[test]
fn media_type_parse() {
    assert_eq!(DppMediaType::parse("application/json"), DppMediaType::Json);
    assert_eq!(
        DppMediaType::parse("application/ld+json; charset=utf-8"),
        DppMediaType::JsonLd
    );
    assert_eq!(DppMediaType::parse("text/html"), DppMediaType::Html);
}

// ── from_accept_header (RFC 9110 q-value parsing) ───────────────────────

#[test]
fn accept_header_html_picks_html() {
    let req = ResolutionRequest::from_accept_header("text/html");
    assert_eq!(req.media_type, Some(DppMediaType::Html));
}

#[test]
fn accept_header_json_ld_picks_json_ld() {
    let req = ResolutionRequest::from_accept_header("application/ld+json");
    assert_eq!(req.media_type, Some(DppMediaType::JsonLd));
}

#[test]
fn accept_header_browser_wildcard_picks_html() {
    // Typical browser: text/html has q=1.0, */* has q=0.8 → html wins.
    let req = ResolutionRequest::from_accept_header(
        "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
    );
    assert_eq!(req.media_type, Some(DppMediaType::Html));
}

#[test]
fn accept_header_q_value_respected() {
    // text/html;q=0.1 loses to application/ld+json;q=0.9
    let req = ResolutionRequest::from_accept_header("text/html;q=0.1, application/ld+json;q=0.9");
    assert_eq!(req.media_type, Some(DppMediaType::JsonLd));
}

#[test]
fn accept_header_wildcard_only_returns_none() {
    let req = ResolutionRequest::from_accept_header("*/*");
    assert_eq!(req.media_type, None);
}

#[test]
fn accept_header_empty_returns_none() {
    let req = ResolutionRequest::from_accept_header("");
    assert_eq!(req.media_type, None);
}

#[test]
fn empty_descriptors_returns_none() {
    let req = ResolutionRequest {
        link_type: None,
        media_type: None,
        access_tier: None,
    };
    assert!(negotiate(&[], &req).is_none());
}
