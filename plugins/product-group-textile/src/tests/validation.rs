//! Tests of the `product-group-textile` module: validation.

use super::super::*;
use dpp_plugin_sdk::traits::PluginComplianceStatus;
use serde_json::json;

fn textile() -> Value {
    json!({
        // 🚨 The schema has required this since v1.3.0 and this fixture
        // omitted it, so every textile test ran against a record the
        // schema would refuse.
        "productIdentifier": {"scheme": "gs1", "gtin": "09506000134352"},
        "fibreComposition": [
            { "fibre": "cotton", "pct": 60.0 },
            { "fibre": "polyester", "pct": 40.0 }
        ],
        "countryOfOrigin": "BD",
        "careInstructions": "Machine wash 40C",
        "chemicalComplianceStandard": "OEKO-TEX 100",
        "recycledContentPct": 30.0
    })
}

#[test]
fn fibre_sum_100_is_compliant() {
    let r = TextilePlugin.calculate_metrics(&textile()).unwrap();
    assert_eq!(r.compliance_status, PluginComplianceStatus::Compliant);
    assert_eq!(r.recycled_content_pct(), Some(30.0));
}

#[test]
fn fibre_sum_off_is_non_compliant() {
    let mut d = textile();
    d["fibreComposition"] = json!([{ "fibre": "cotton", "pct": 50.0 }]);
    assert_eq!(
        TextilePlugin
            .calculate_metrics(&d)
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::NonCompliant
    );
}

#[test]
fn a_textile_without_an_identifier_is_refused() {
    let mut d = textile();
    d.as_object_mut()
        .expect("object")
        .remove("productIdentifier");
    match TextilePlugin.validate_input(&d) {
        Err(PluginError::ValidationErrors(errors)) => assert!(
            errors.iter().any(|e| e.field == "/productIdentifier"),
            "the refusal must name the field: {errors:?}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn missing_fibre_composition_fails_validation() {
    let mut d = textile();
    d.as_object_mut().unwrap().remove("fibreComposition");
    assert!(TextilePlugin.validate_input(&d).is_err());
}

#[test]
fn out_of_range_fibre_pcts_are_non_compliant() {
    // Sums to 100 but neither percentage is physically valid.
    let mut d = textile();
    d["fibreComposition"] = json!([
        { "fibre": "cotton", "pct": 150.0 },
        { "fibre": "wool", "pct": -50.0 }
    ]);
    assert_eq!(
        TextilePlugin
            .calculate_metrics(&d)
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::NonCompliant
    );
}

#[test]
fn fibre_entry_missing_pct_is_non_compliant() {
    // One entry has no pct — an incomplete declaration, not a 100% cotton.
    let mut d = textile();
    d["fibreComposition"] = json!([
        { "fibre": "cotton" },
        { "fibre": "wool", "pct": 100.0 }
    ]);
    assert_eq!(
        TextilePlugin
            .calculate_metrics(&d)
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::NonCompliant
    );
}

#[test]
fn negative_carbon_footprint_fails_validation() {
    let mut d = textile();
    d["carbonFootprintKgCo2e"] = json!(-10.0);
    assert!(TextilePlugin.validate_input(&d).is_err());
}
