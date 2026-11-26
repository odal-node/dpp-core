//! Tests of the `vocabulary` module.

use super::super::*;

#[test]
fn every_known_link_type_round_trips_via_canonical_uri() {
    let all = [
        Gs1LinkType::ProductInformationPage,
        Gs1LinkType::ElectronicLeaflet,
        Gs1LinkType::SustainabilityInfo,
        Gs1LinkType::RecyclingInfo,
        Gs1LinkType::MasterData,
        Gs1LinkType::CertificationInfo,
        Gs1LinkType::Instructions,
        Gs1LinkType::SafetyInfo,
        Gs1LinkType::Traceability,
        Gs1LinkType::DigitalProductPassport,
        Gs1LinkType::Predecessor,
        Gs1LinkType::Successor,
        Gs1LinkType::HasComponent,
        Gs1LinkType::IsComponentOf,
    ];
    for lt in all {
        let uri = lt.as_gs1_uri();
        assert_eq!(Gs1LinkType::parse(uri), lt, "canonical URI must round-trip");
    }
}

#[test]
fn shorthand_and_full_uris_parse_equivalently() {
    assert_eq!(
        Gs1LinkType::parse("gs1:dpp"),
        Gs1LinkType::DigitalProductPassport
    );
    assert_eq!(
        Gs1LinkType::parse("gs1:pip"),
        Gs1LinkType::ProductInformationPage
    );
    assert_eq!(
        Gs1LinkType::parse("gs1:safetyInfo"),
        Gs1LinkType::SafetyInfo
    );
}

#[test]
fn unknown_link_type_becomes_custom() {
    let custom = Gs1LinkType::parse("https://example.com/voc/warranty");
    assert_eq!(
        custom,
        Gs1LinkType::Custom("https://example.com/voc/warranty".to_owned())
    );
    assert_eq!(custom.as_gs1_uri(), "https://example.com/voc/warranty");
}
