//! A passport whose every nested envelope object has every field set.
//!
//! Shared by `jsonld_envelope_terms.rs`, which holds the context to what comes out
//! of serialising it, and `jsonld_oracle_corpus.rs`, which hands the framed form
//! to an independent JSON-LD processor. Each includes this file with `#[path]`.
//!
//! # Why every object is a struct literal
//!
//! The terms for these objects are a table (`dpp_vc`'s `envelope_terms`), because
//! the objects are Rust types and there is no schema to generate them from. A
//! table is only as good as what compares it with the types, and a fixture that
//! sets the fields its author remembered proves nothing about the ones added
//! later: half of them are `skip_serializing_if`, so a `None` is a key that
//! never appears. Written as a literal, a type that gains a field stops compiling
//! here until the field is named, and the next thing to fail is the term.
//!
//! What a literal cannot catch is a new *variant* of an enum that carries data,
//! since a variant this file never builds is never serialised. The one such enum
//! on the envelope is [`ResponsibilityBasis`], which lists its own members for
//! exactly this use, so [`bases`] builds from that rather than from this file.

use std::collections::BTreeMap;

use chrono::Utc;
use dpp_domain::passport::{
    ComponentRef, DerivationRef, PassportRef, Quantity, SecondLifeOperation,
};
use dpp_domain::seal::{SealConformanceLevel, SealFormat, SealedEnvelope};
use dpp_domain::{
    CarbonFootprint, CarbonFootprintClass, ComplianceFinding, ComplianceResult, ComplianceStatus,
    FacilitySnapshot, InstrumentRef, LifecycleStage, LintFinding, LintResult, LintSeverity,
    ManufacturerInfo, MaterialEntry, OperatorRole, Passport, RecordedBasis, RepairCriterion,
    RepairabilityScore, ResponsibilityBasis, ResponsibleOperator, ResponsibleOperatorSnapshot,
    SystemBoundary,
};
use serde_json::json;

/// Every basis this build models, each in a form that serialises with all its
/// members set.
pub fn bases() -> Vec<ResponsibilityBasis> {
    ResponsibilityBasis::all()
        .into_iter()
        .map(|basis| match basis {
            ResponsibilityBasis::OtherUnionLaw { .. } => ResponsibilityBasis::OtherUnionLaw {
                citation: "Article 7 of Regulation (EU) 2017/745".into(),
            },
            other => other,
        })
        .collect()
}

/// Set every nested envelope object of `passport`, each with every field set.
pub fn populate_envelope(passport: &mut Passport, basis: ResponsibilityBasis) {
    let at = Utc::now();
    let reference = || PassportRef {
        uri: "https://example.com/p/00000000-0000-7000-8000-000000000001".into(),
        public_jws_hash: "sha256-0000".into(),
    };

    passport.applicable_instruments = vec![InstrumentRef {
        instrument: "esprRegulation".into(),
        recorded: RecordedBasis::Catalog,
    }];

    passport.manufacturer = ManufacturerInfo {
        name: "Acme Manufacturing GmbH".into(),
        address: "Hauptstraße 1, 10115 Berlin, DE".into(),
        registered_trade_name: Some("Acme".into()),
        electronic_address: Some("https://acme.example.com/contact".into()),
        country: Some("DE".into()),
        did_web_url: Some("https://acme.example.com/.well-known/did.json".into()),
    };

    passport.materials = vec![MaterialEntry {
        name: "Primary material".into(),
        weight_kg: 1.5,
        recycled_pct: Some(20.0),
        country_of_origin: Some("DE".into()),
    }];

    passport.co2e_per_unit = Some(CarbonFootprint {
        value_kg: 12.0,
        lifecycle_stage: Some(LifecycleStage::CradleToGate),
        system_boundary: Some(SystemBoundary::En15804),
        methodology_ref: Some("EN 15804".into()),
        performance_class: Some(CarbonFootprintClass::new("B").expect("a short label")),
    });

    passport.repairability_score = Some(RepairabilityScore {
        overall: 6.0,
        criteria: vec![RepairCriterion {
            name: "Disassembly".into(),
            score: 6.0,
            weight: 1.0,
        }],
    });

    let finding = ComplianceFinding {
        code: "X-1".into(),
        field: "materials".into(),
        message: "a finding".into(),
    };
    passport.compliance_result = Some(ComplianceResult {
        co2e_score: Some(12.0),
        repairability_index: Some(6.0),
        recycled_content_pct: Some(20.0),
        compliance_status: ComplianceStatus::Compliant,
        violations: vec![finding.clone()],
        warnings: vec![finding],
        ruleset_version: Some("1.0.0".into()),
        assessed_at: Some(at),
        // Whatever a calculation recorded: keys no schema fixes.
        receipt: Some(json!({ "inputs": { "anyKey": 1 }, "steps": [{ "free": "form" }] })),
    });

    passport.lint_result = Some(LintResult {
        pack_version: "1.0.0".into(),
        findings: vec![LintFinding {
            code: "L-1".into(),
            field: "materials".into(),
            severity: LintSeverity::Warning,
            message: "a finding".into(),
        }],
        assessed_at: at,
    });

    passport.disclosure_signatures = BTreeMap::from([
        (
            "usageHistory.negativeEvents".to_owned(),
            "eyJ...".to_owned(),
        ),
        ("materials".to_owned(), "eyJ...".to_owned()),
    ]);

    passport.derived_from = vec![DerivationRef {
        reference: reference(),
        operation: SecondLifeOperation::Remanufacturing,
    }];

    passport.component_refs = vec![ComponentRef {
        reference: reference(),
        quantity: Some(Quantity {
            value: 2.0,
            unit: Some("pcs".into()),
        }),
        role: Some("cell".into()),
    }];

    passport.responsible_operator = Some(ResponsibleOperatorSnapshot {
        operator: ResponsibleOperator {
            did: "did:web:operator.example.com".into(),
            name: "Operator GmbH".into(),
            role: OperatorRole::Manufacturer,
            eu_operator_id: Some("DE-EORI-0001".into()),
            eu_operator_id_scheme: Some("EORI".into()),
            country: "DE".into(),
            registered_trade_name: Some("Operator".into()),
            postal_address: Some("Hauptstraße 2, 10115 Berlin, DE".into()),
            electronic_address: Some("https://operator.example.com/contact".into()),
        },
        basis,
    });

    passport.facility = Some(FacilitySnapshot {
        scheme: "GLN".into(),
        value: "9506000134352".into(),
        name: "Berlin plant".into(),
        country: "DE".into(),
        address: Some("Werkstraße 3, 10115 Berlin, DE".into()),
    });

    passport.seal = Some(SealedEnvelope {
        format: SealFormat::Jades,
        seal_value: "eyJ...".into(),
        signing_cert_ref: Some("cert-1".into()),
        conformance_level: Some(SealConformanceLevel::BaselineB),
        sealed_at: at,
        placeholder: false,
    });
}
