//! The disclosure's versioned schemas: what they accept, and that every version is
//! frozen against a document.

use semver::Version;
use serde_json::{Value, json};

use crate::catalog::ProductGroupCatalog;
use crate::schemas::VersionedSchemaRegistry;
use crate::unsold_goods::{DISCLOSURE_SCHEMA_KEY, UnsoldGoodsReport, document_schemas};

fn v2_0() -> Version {
    "2.0.0".parse().unwrap()
}

/// A disclosure that validates, for a test to break one thing in.
fn disclosure() -> Value {
    json!({
        "entity": {
            "name": "Example Retail Group SA",
            "identifier": { "type": "euid", "value": "LUB123456789" },
            "scope": { "type": "standalone" }
        },
        "financialYear": { "start": "2027-01-01", "end": "2027-12-31" },
        "lines": [{
            "cnCategories": ["6203"],
            "description": "Men's suits and trousers",
            "unitsDiscarded": { "value": 1200 },
            "weightKg": { "value": 430, "estimated": true },
            "packagingIncluded": false,
            "reason": "damagedOrContaminated",
            "treatment": {
                "preparingForReusePct": 20, "recyclingPct": 50,
                "otherRecoveryPct": 20, "disposalPct": 5, "unknownPct": 5
            }
        }],
        "measuresTaken": "Pre-season demand forecasting.",
        "measuresPlanned": "Twelve-week donation window."
    })
}

fn validates(body: &Value) -> bool {
    document_schemas()
        .validate(DISCLOSURE_SCHEMA_KEY, &v2_0(), body)
        .is_ok()
}

#[test]
fn conformance_unsold_goods_v2_valid() {
    assert!(validates(&disclosure()));
}

/// The reason vocabulary is Del. Reg. (EU) 2026/296 Art. 2's derogation list.
/// `end_of_season` was ours and is not a derogation at all — a disclosure using
/// it claimed a lawful destruction the act does not permit.
#[test]
fn conformance_unsold_goods_v2_rejects_a_reason_outside_article_2() {
    let mut data = disclosure();
    data["lines"][0]["reason"] = json!("end_of_season");
    assert!(!validates(&data));
}

/// Art. 3 delimits by CN chapter or heading; a product's own 6/8/10-digit code
/// is a different level of the nomenclature and files a whole chapter's goods
/// under one article.
#[test]
fn conformance_unsold_goods_v2_rejects_a_full_commodity_code_as_a_category() {
    let mut data = disclosure();
    data["lines"][0]["cnCategories"] = json!(["62034231"]);
    assert!(!validates(&data));
}

/// Art. 24 reaches discarded unsold consumer products generally; Art. 25's
/// destruction ban reaches Annex VII's apparel and footwear. Nothing in the schema
/// asks for Annex VII scope, so a disclosure of refrigerators — Annex II heading
/// 8418, nowhere near Annex VII — is as valid as one of trousers.
#[test]
fn a_disclosure_outside_annex_vii_scope_validates() {
    let mut data = disclosure();
    data["lines"][0]["cnCategories"] = json!(["8418"]);
    assert!(validates(&data));
}

/// Annex I's first rows identify the undertaking and the financial year it reports
/// on, so a disclosure naming neither says nothing about whose figures these are or
/// for when.
#[test]
fn a_disclosure_without_its_header_rows_is_rejected() {
    for field in ["entity", "financialYear"] {
        let mut data = disclosure();
        data.as_object_mut().expect("object").remove(field);
        assert!(
            !validates(&data),
            "a disclosure without {field} was accepted"
        );
    }
}

/// Annex I's table has a body, and a disclosure with none says nothing.
#[test]
fn a_disclosure_with_no_lines_is_rejected() {
    let mut data = disclosure();
    data["lines"] = json!([]);
    assert!(!validates(&data));
}

/// The disclosure is not a product group, so it is neither listed with the
/// product-group schemas nor present in the catalog.
#[test]
fn the_disclosure_is_not_a_product_group() {
    assert!(
        VersionedSchemaRegistry::new()
            .versions_for(DISCLOSURE_SCHEMA_KEY)
            .is_empty()
    );
    assert!(
        ProductGroupCatalog::new()
            .get(DISCLOSURE_SCHEMA_KEY)
            .is_none()
    );
}

/// One family under one key, every version of it, and nothing else in the
/// registry.
#[test]
fn the_disclosure_registry_holds_the_disclosure_schema_and_nothing_else() {
    let schemas = document_schemas();
    assert_eq!(schemas.product_groups(), [DISCLOSURE_SCHEMA_KEY]);
    assert_eq!(schemas.versions_for(DISCLOSURE_SCHEMA_KEY), [&v2_0()]);
}

fn fixture(version: &str) -> Option<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/document-schemas")
        .join(DISCLOSURE_SCHEMA_KEY)
        .join(format!("v{version}.json"));
    let raw = std::fs::read_to_string(path).ok()?;
    Some(serde_json::from_str(&raw).expect("a frozen fixture must be valid JSON"))
}

/// A declared version with no frozen document is an unguarded version. A document
/// written under a version must keep validating against it and keep reading as the
/// current type, unchanged on the way back out.
///
/// A fixture is written once and never regenerated: one regenerated from the current
/// schema would agree with it by construction and could not catch anything.
#[test]
fn every_declared_version_has_a_frozen_document_that_still_validates_and_reads() {
    let schemas = document_schemas();
    let versions: Vec<String> = schemas
        .versions_for(DISCLOSURE_SCHEMA_KEY)
        .iter()
        .map(|v| v.to_string())
        .collect();
    assert!(!versions.is_empty());

    for version in versions {
        let document = fixture(&version)
            .unwrap_or_else(|| panic!("no frozen document for v{version}: add one"));

        schemas
            .validate_strict(DISCLOSURE_SCHEMA_KEY, &version, &document)
            .unwrap_or_else(|e| {
                panic!("the frozen v{version} document no longer validates: {e:?}")
            });

        let report: UnsoldGoodsReport = serde_json::from_value(document.clone())
            .unwrap_or_else(|e| panic!("the frozen v{version} document no longer reads: {e}"));
        assert_eq!(
            serde_json::to_value(&report).unwrap(),
            document,
            "v{version} does not round-trip"
        );
    }
}
