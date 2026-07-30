//! Example: parse a GS1 Digital Link and negotiate a link type.
//!
//! Run with: `cargo run --example parse_and_negotiate`

use dpp_digital_link::{
    DigitalLink, DppMediaType, Gs1LinkType, LinkDescriptor, ResolutionRequest, negotiate,
};
use dpp_domain::Disclosure;

fn main() {
    println!("=== GS1 Digital Link Parsing ===\n");

    let uri = "https://id.odal-node.io/01/09506000134352/21/SN-2026-001";
    let link = DigitalLink::parse(uri).unwrap();

    println!("Parsed: {uri}");
    println!("  Resolver: {}", link.resolver_base);
    println!("  GTIN (AI 01): {}", link.gtin);
    println!(
        "  Serial (AI 21): {}",
        link.serial.as_deref().unwrap_or("—")
    );
    println!("  Batch (AI 10): {}", link.batch.as_deref().unwrap_or("—"));

    let uri_batch = "https://id.odal-node.io/01/09506000134352/10/LOT-Q2-2026/21/UNIT-042";
    let link_batch = DigitalLink::parse(uri_batch).unwrap();
    println!("\nParsed: {uri_batch}");
    println!("  GTIN: {}", link_batch.gtin);
    println!("  Batch: {}", link_batch.batch.as_deref().unwrap_or("—"));
    println!("  Serial: {}", link_batch.serial.as_deref().unwrap_or("—"));

    let built = link.build();
    println!("\nRebuilt URI: {built}");

    println!("\n=== Link-Type Negotiation ===\n");

    let descriptors = vec![
        LinkDescriptor {
            link_type: Gs1LinkType::DigitalProductPassport,
            media_type: DppMediaType::Json,
            disclosure: Disclosure::Public,
            href: "https://api.odal-node.io/dpp/09506000134352/data".into(),
            title: Some("DPP JSON".into()),
            language: None,
        },
        LinkDescriptor {
            link_type: Gs1LinkType::DigitalProductPassport,
            media_type: DppMediaType::JsonLd,
            disclosure: Disclosure::Public,
            href: "https://api.odal-node.io/dpp/09506000134352/data.jsonld".into(),
            title: Some("DPP JSON-LD".into()),
            language: None,
        },
        LinkDescriptor {
            link_type: Gs1LinkType::ProductInformationPage,
            media_type: DppMediaType::Html,
            disclosure: Disclosure::Public,
            href: "https://passport.odal-node.io/09506000134352".into(),
            title: Some("Human-readable passport".into()),
            language: Some("en".into()),
        },
    ];

    let request = ResolutionRequest {
        link_type: Some(Gs1LinkType::DigitalProductPassport),
        media_type: Some(DppMediaType::Json),
        audience: None,
    };

    let resolved = negotiate(&descriptors, &request);
    println!("Negotiation request: JSON Digital Product Passport");
    println!("  Resolved: {}", resolved.unwrap().href);
}
