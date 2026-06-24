//! Example: the four persistent identifiers the EU Central Registry requires
//! under ESPR Article 13, and their structural validation.
//!
//! `dpp-registry` is wasm32-safe — pure types, no I/O. The platform repo
//! provides the HTTP transport that submits these to the registry.
//!
//! Run with: `cargo run --example registry_identifiers -p dpp-registry`

use dpp_registry::{FacilityIdentifier, OperatorIdentifier, ProductIdentifier};

fn main() {
    println!("=== Product identifier (GTIN-14, mod-10 checked) ===\n");

    let product = ProductIdentifier {
        scheme: "gtin".into(),
        value: "09506000134352".into(),
        label: Some("PowerCell EV Module 4680".into()),
    };
    println!("  valid GTIN   -> {:?}", product.validate());

    let bad_product = ProductIdentifier {
        scheme: "gtin".into(),
        value: "09506000134351".into(), // wrong check digit
        label: None,
    };
    match bad_product.validate() {
        Ok(()) => println!("  bad GTIN     -> unexpectedly OK"),
        Err(e) => println!("  bad GTIN     -> {e}"),
    }

    println!("\n=== Facility identifier (country code validated) ===\n");

    let facility = FacilityIdentifier {
        scheme: "gln".into(),
        value: "4012345000009".into(),
        name: Some("München Cell Plant".into()),
        country: "DE".into(),
        address: Some("Industriestraße 7, München".into()),
    };
    println!("  DE facility  -> {:?}", facility.validate());

    let bad_facility = FacilityIdentifier {
        scheme: "gln".into(),
        value: "x".into(),
        name: None,
        country: "EU".into(), // reserved by EC, not a valid ISO 3166-1 alpha-2
        address: None,
    };
    match bad_facility.validate() {
        Ok(()) => println!("  EU facility  -> unexpectedly OK"),
        Err(e) => println!("  EU facility  -> {e}"),
    }

    println!("\n=== Economic operator identifier ===\n");

    let operator = OperatorIdentifier {
        scheme: "vat".into(),
        value: "DE123456789".into(),
        name: "Volt Dynamics GmbH".into(),
        country: "DE".into(),
        did: Some("did:web:voltdynamics.example.com".into()),
    };
    println!("  operator     -> {:?}", operator.validate());

    // Identifiers serialise to camelCase JSON for the registry envelope.
    println!(
        "\n  product as JSON: {}",
        serde_json::to_string(&product).unwrap()
    );
}
