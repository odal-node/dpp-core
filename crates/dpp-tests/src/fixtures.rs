//! Shared fixture builders for the integration tests in `tests/` (and the
//! `dpp-benches` crate, which depends on this library as a dev-dependency).
//!
//! These are the sector-agnostic envelope / actor shapes that were hand-rolled
//! at multiple call sites before this module existed.

use chrono::Utc;
use dpp_domain::{
    CarbonFootprint, ManufacturerInfo, MaterialEntry, OperatorRole, Passport, PassportId,
    PassportStatus, RepairabilityScore, ResponsibleOperator, Sector, SectorData,
};
use dpp_vc::{CredentialRole, DppCredentialSubject};

/// A base passport with the sector-agnostic fields populated so the five core
/// AAS submodels (identification, manufacturer, environmental, materials,
/// repairability) all exercise their optional branches. Callers override
/// individual fields via struct-update syntax for scenario-specific values.
pub fn base_passport(sector: Sector, sector_data: SectorData, schema_version: &str) -> Passport {
    // A caller-supplied version is honoured only if it is one this build knows;
    // otherwise the sector's current version is used.
    //
    // Disclosure classes are now sourced from the declared schema version, so a
    // fixture that declares an older version while carrying data built from the
    // *current* typed structs classifies none of the newer fields — and the
    // masking backstop strips them, leaving a submodel with nothing in it. That
    // is the fixture being wrong rather than the code, and it was invisible
    // while the catalog's disclosure map ignored versions entirely.
    let catalog = dpp_domain::SectorCatalog::new();
    let schema_version = catalog.get(sector.catalog_key()).map_or_else(
        || schema_version.to_owned(),
        |d| {
            if d.schema_versions.iter().any(|v| v == schema_version) {
                d.current_schema_version.clone()
            } else {
                schema_version.to_owned()
            }
        },
    );
    let schema_version = schema_version.as_str();
    let now = Utc::now();
    Passport {
        id: PassportId::new(),
        batch_id: Some("LOT-X-0001".into()),
        product_name: format!("{} reference product", sector.catalog_key()),
        sector,
        manufacturer: ManufacturerInfo {
            name: "Acme Manufacturing GmbH".into(),
            address: "Hauptstraße 1, 10115 Berlin, DE".into(),
            did_web_url: Some("https://acme.example.com/.well-known/did.json".into()),
        },
        materials: vec![MaterialEntry {
            name: "Primary material".into(),
            weight_kg: 1.5,
            recycled_pct: Some(20.0),
            country_of_origin: Some("DE".into()),
        }],
        co2e_per_unit: Some(CarbonFootprint::from_kg(12.0)),
        repairability_score: Some(RepairabilityScore::from_scalar(6.0)),
        compliance_result: None,
        lint_result: None,
        sector_data: Some(sector_data),
        status: PassportStatus::Draft,
        qr_code_url: None,
        jws_signature: None,
        public_jws_signature: None,
        disclosure_signatures: Default::default(),
        created_at: now,
        updated_at: now,
        published_at: None,
        placed_on_market_date: None,
        schema_version: schema_version.into(),
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        parent_passport_ref: None,
        component_refs: Vec::new(),
        retention_until: None,
        product_id: None,
        commodity_code: None,
        operator_identifier: None,
        facility: None,
        seal: None,
    }
}

/// A [`ResponsibleOperator`] for transfer-of-responsibility / provenance tests.
pub fn make_operator(
    did: &str,
    name: &str,
    role: OperatorRole,
    country: &str,
) -> ResponsibleOperator {
    ResponsibleOperator {
        did: did.into(),
        name: name.into(),
        role,
        eu_operator_id: None,
        eu_operator_id_scheme: None,
        country: country.into(),
    }
}

/// A [`DppCredentialSubject`] for audience / verifiable-credential tests.
pub fn make_subject(
    did: &str,
    name: &str,
    role: CredentialRole,
    sectors: Vec<String>,
) -> DppCredentialSubject {
    DppCredentialSubject {
        id: did.into(),
        name: name.into(),
        role,
        country: "DE".into(),
        sectors,
        product_categories: vec![],
    }
}
