//! Shared fixture builders for the integration tests in `tests/` (and the
//! `dpp-benches` crate, which depends on this library as a dev-dependency).
//!
//! These are the product group-agnostic envelope / actor shapes that were hand-rolled
//! at multiple call sites before this module existed.

use chrono::Utc;
use dpp_domain::{
    CarbonFootprint, ManufacturerInfo, MaterialEntry, OperatorRole, Passport, PassportId,
    PassportStatus, ProductGroup, ProductGroupData, RepairabilityScore, ResponsibleOperator,
};
use dpp_vc::{CredentialRole, DppCredentialSubject};

/// A base passport with the product group-agnostic fields populated so the five core
/// AAS submodels (identification, manufacturer, environmental, materials,
/// repairability) all exercise their optional branches. Callers override
/// individual fields via struct-update syntax for scenario-specific values.
pub fn base_passport(
    product_group: ProductGroup,
    product_group_data: ProductGroupData,
    schema_version: &str,
) -> Passport {
    // A caller-supplied version is honoured only if it is one this build knows;
    // otherwise the product group's current version is used.
    //
    // Disclosure classes are now sourced from the declared schema version, so a
    // fixture that declares an older version while carrying data built from the
    // *current* typed structs classifies none of the newer fields — and the
    // masking backstop strips them, leaving a submodel with nothing in it. That
    // is the fixture being wrong rather than the code, and it was invisible
    // while the catalog's disclosure map ignored versions entirely.
    let catalog = dpp_domain::ProductGroupCatalog::new();
    let schema_version = catalog.get(product_group.catalog_key()).map_or_else(
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
        product_name: format!("{} reference product", product_group.catalog_key()),
        applicable_instruments: dpp_domain::InstrumentCatalog::new()
            .instrument_refs_for(product_group.catalog_key()),
        granularity: None,
        product_group,
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
        product_group_data: Some(product_group_data),
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
        derived_from: Vec::new(),
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
    product_groups: Vec<String>,
) -> DppCredentialSubject {
    DppCredentialSubject {
        id: did.into(),
        name: name.into(),
        role,
        country: "DE".into(),
        product_groups,
        product_categories: vec![],
    }
}

/// A minimal, well-formed unsold-goods disclosure in the Annex I shape of
/// Commission Implementing Regulation (EU) 2026/2.
///
/// One line, a treatment split totalling 100, and a CN heading outside Annex II
/// so the depth lint stays quiet.
#[must_use]
pub fn unsold_goods_report() -> dpp_domain::UnsoldGoodsReport {
    use chrono::NaiveDate;
    use dpp_domain::{
        CnCategory, DiscardReason, DiscardedProductLine, DiscardedQuantity, DisclosingEntity,
        DisclosureScope, FinancialYear, LegalEntityIdentifier, UnsoldGoodsReport,
        WasteTreatmentSplit,
    };

    UnsoldGoodsReport {
        entity: DisclosingEntity {
            name: "Example Retail Group SA".into(),
            identifier: LegalEntityIdentifier::Euid {
                value: "LUB123456789".into(),
            },
            scope: DisclosureScope::Standalone,
        },
        financial_year: FinancialYear {
            start: NaiveDate::from_ymd_opt(2027, 1, 1).expect("valid date"),
            end: NaiveDate::from_ymd_opt(2027, 12, 31).expect("valid date"),
        },
        lines: vec![DiscardedProductLine {
            cn_categories: vec![CnCategory::parse("6203").expect("valid CN heading")],
            description: "Men's suits, ensembles, jackets and trousers".into(),
            units_discarded: DiscardedQuantity::measured(1_200),
            weight_kg: DiscardedQuantity::estimated(430),
            packaging_included: false,
            reason: DiscardReason::DamagedOrContaminated,
            reason_detail: None,
            treatment: WasteTreatmentSplit {
                preparing_for_reuse_pct: 20,
                recycling_pct: 50,
                other_recovery_pct: 20,
                disposal_pct: 5,
                unknown_pct: 5,
            },
        }],
        measures_taken: "Introduced pre-season demand forecasting across all lines.".into(),
        measures_planned: "Extending the donation offer window to twelve weeks.".into(),
    }
}
