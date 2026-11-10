//! The disclosure's versioned schemas: what they accept, and that every version is
//! frozen against a document.

use semver::Version;
use serde_json::{Value, json};

use crate::catalog::ProductGroupCatalog;
use crate::schemas::VersionedSchemaRegistry;
use crate::test_support::sample_discard_record;
use crate::unsold_goods::{
    DISCLOSURE_SCHEMA_KEY, DiscardRecord, RECORD_SCHEMA_KEY, UnsoldGoodsReport, document_schemas,
};

fn v2_0() -> Version {
    "2.0.0".parse().unwrap()
}

/// The version that stopped requiring a reason on a line.
fn v2_1() -> Version {
    "2.1.0".parse().unwrap()
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
    validates_as(&v2_0(), body)
}

/// Whether `body` validates against the disclosure schema at `version`.
fn validates_as(version: &Version, body: &Value) -> bool {
    document_schemas()
        .validate(DISCLOSURE_SCHEMA_KEY, version, body)
        .is_ok()
}

/// A disclosure whose one line carries no `reason`, as a discard outside the
/// destruction ban's reach has none.
fn disclosure_without_a_reason() -> Value {
    let mut data = disclosure();
    data["lines"][0]
        .as_object_mut()
        .expect("object")
        .remove("reason");
    data
}

#[test]
fn conformance_unsold_goods_v2_valid() {
    assert!(validates(&disclosure()));
}

/// Annex I note (h) asks for the reason "where applicable", and the derogations
/// reach only the destruction of Annex VII products, so a line for any other
/// discard has no Art. 2 point. v2.1.0 stops requiring one.
#[test]
fn conformance_unsold_goods_v2_1_accepts_a_line_with_no_derogation() {
    assert!(validates_as(&v2_1(), &disclosure_without_a_reason()));
}

/// v2.0.0 is published and stays as it was: a document stored under it was
/// validated against a required reason.
#[test]
fn conformance_unsold_goods_v2_0_still_requires_a_reason() {
    assert!(!validates_as(&v2_0(), &disclosure_without_a_reason()));
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

/// The two families of the module under their own keys, every version of each, and
/// nothing else in the registry.
#[test]
fn the_registry_holds_the_modules_two_families_and_nothing_else() {
    let schemas = document_schemas();
    assert_eq!(
        schemas.product_groups(),
        [RECORD_SCHEMA_KEY, DISCLOSURE_SCHEMA_KEY]
    );
    assert_eq!(
        schemas.versions_for(DISCLOSURE_SCHEMA_KEY),
        [&v2_0(), &v2_1()]
    );
    assert_eq!(
        schemas.versions_for(RECORD_SCHEMA_KEY),
        ["1.0.0".parse::<Version>().as_ref().unwrap()]
    );
}

/// The frozen document for a version of a family, if there is one.
fn fixture(key: &str, version: &str) -> Option<Value> {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/document-schemas")
        .join(key)
        .join(format!("v{version}.json"));
    let raw = std::fs::read_to_string(path).ok()?;
    Some(serde_json::from_str(&raw).expect("a frozen fixture must be valid JSON"))
}

/// Every version of the family `key` has a frozen document that still validates
/// against it, and reads as the current type `T`, unchanged on the way back out.
fn frozen_documents_still_validate_and_read<T>(key: &str)
where
    T: serde::de::DeserializeOwned + serde::Serialize,
{
    let schemas = document_schemas();
    let versions: Vec<String> = schemas
        .versions_for(key)
        .iter()
        .map(|v| v.to_string())
        .collect();
    assert!(!versions.is_empty());

    for version in versions {
        let document = fixture(key, &version)
            .unwrap_or_else(|| panic!("no frozen {key} document for v{version}: add one"));

        schemas
            .validate_strict(key, &version, &document)
            .unwrap_or_else(|e| {
                panic!("the frozen {key} v{version} document no longer validates: {e:?}")
            });

        let read: T = serde_json::from_value(document.clone()).unwrap_or_else(|e| {
            panic!("the frozen {key} v{version} document no longer reads: {e}")
        });
        assert_eq!(
            serde_json::to_value(&read).unwrap(),
            document,
            "{key} v{version} does not round-trip"
        );
    }
}

/// A declared version with no frozen document is an unguarded version. A document
/// written under a version must keep validating against it and keep reading as the
/// current type, unchanged on the way back out.
///
/// A fixture is written once and never regenerated: one regenerated from the current
/// schema would agree with it by construction and could not catch anything.
#[test]
fn every_declared_version_has_a_frozen_document_that_still_validates_and_reads() {
    frozen_documents_still_validate_and_read::<UnsoldGoodsReport>(DISCLOSURE_SCHEMA_KEY);
    frozen_documents_still_validate_and_read::<DiscardRecord>(RECORD_SCHEMA_KEY);
}

/// Whether `body` validates against the discard-record schema v1.0.0.
fn records_validate_as_v1(body: &Value) -> bool {
    document_schemas()
        .validate(RECORD_SCHEMA_KEY, &"1.0.0".parse().unwrap(), body)
        .is_ok()
}

/// A record as the type writes it is a record the schema admits, with and without a
/// derogation: the schema is the contract for what the type produces.
#[test]
fn a_record_as_the_type_writes_it_validates() {
    let with_reason = serde_json::to_value(sample_discard_record()).unwrap();
    assert!(records_validate_as_v1(&with_reason), "{with_reason}");

    let without = serde_json::to_value(DiscardRecord {
        reason: None,
        reason_label: Some("Surplus after a discontinued range".to_owned()),
        ..sample_discard_record()
    })
    .unwrap();
    assert!(records_validate_as_v1(&without), "{without}");
}

/// What the schema refuses that the type reads. The type keeps a malformed record
/// readable so it can be reported; the schema is where it is turned away at the
/// door, and each of these once went through the type unchallenged.
#[test]
fn a_record_offered_for_storage_is_held_to_its_shape() {
    let record = || serde_json::to_value(sample_discard_record()).unwrap();
    let broken = |edit: &dyn Fn(&mut Value)| {
        let mut value = record();
        edit(&mut value);
        value
    };

    let refused = [
        (
            "a CN code that is not digits",
            broken(&|v| v["cnCodes"] = json!(["abc"])),
        ),
        (
            "a CN code of five digits",
            broken(&|v| v["cnCodes"] = json!(["62034"])),
        ),
        (
            "a CN category in place of a code",
            broken(&|v| v["cnCodes"] = json!(["6203"])),
        ),
        ("no CN code", broken(&|v| v["cnCodes"] = json!([]))),
        (
            "a zero weight",
            broken(&|v| v["weight"]["grams"] = json!(0)),
        ),
        (
            "a negative unit count",
            broken(&|v| v["units"]["value"] = json!(-1)),
        ),
        (
            "a reason outside Art. 2",
            broken(&|v| v["reason"] = json!("end_of_season")),
        ),
        (
            "a label of 121 characters",
            broken(&|v| v["reasonLabel"] = json!("x".repeat(121))),
        ),
        ("an empty label", broken(&|v| v["reasonLabel"] = json!(""))),
        (
            "a treatment outside the five",
            broken(&|v| v["treatment"] = json!("landfill")),
        ),
        (
            "a misspelt property",
            broken(&|v| v["reasonLabell"] = json!("typo")),
        ),
        (
            "no receiver",
            broken(&|v| {
                v.as_object_mut().unwrap().remove("receiver");
            }),
        ),
        (
            "a date that is not one",
            broken(&|v| v["discardedOn"] = json!("14/03/2027")),
        ),
    ];
    for (why, value) in refused {
        assert!(!records_validate_as_v1(&value), "accepted {why}: {value}");
    }
}
