//! Shared fixture builders for the integration tests in `tests/` (and the
//! `dpp-benches` crate, which depends on this library as a dev-dependency).
//!
//! These are the sector-agnostic envelope / actor shapes hand-rolled at
//! multiple call sites before this module existed — see
//! `docs/audit/dpp-core/audit/redundancy-optimization-backlog-2026-07-20.md`.

use chrono::Utc;
use dpp_crypto::{CredentialRole, DppCredentialSubject};
use dpp_domain::{
    CarbonFootprint, ManufacturerInfo, MaterialEntry, OperatorRole, Passport, PassportId,
    PassportStatus, RepairabilityScore, ResponsibleOperator, Sector, SectorData,
};

/// A base passport with the sector-agnostic fields populated so the five core
/// AAS submodels (identification, manufacturer, environmental, materials,
/// repairability) all exercise their optional branches. Callers override
/// individual fields via struct-update syntax for scenario-specific values.
pub fn base_passport(sector: Sector, sector_data: SectorData, schema_version: &str) -> Passport {
    let now = Utc::now();
    Passport {
        id: PassportId::new(),
        batch_id: Some("LOT-X-0001".into()),
        product_name: format!("{} reference product", sector.catalog_key()),
        sector,
        product_category: None,
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
        created_at: now,
        updated_at: now,
        published_at: None,
        schema_version: schema_version.into(),
        retention_locked: false,
        version: 1,
        supersedes_id: None,
        parent_passport_ref: None,
        component_refs: Vec::new(),
        retention_until: None,
        product_id: None,
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
        country: country.into(),
    }
}

/// A [`DppCredentialSubject`] for access-tier / verifiable-credential tests.
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
