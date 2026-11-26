//! Tests of the `product-group-toy` module: validation.

use super::super::*;
use serde_json::json;

fn valid() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "ageGroup": "3-6",
        "primaryMaterial": "wood",
        "ceMarking": true,
        "countryOfOrigin": "DE"
    })
}

#[test]
fn ce_marked_toy_is_not_assessed() {
    assert_eq!(
        ToyPlugin
            .calculate_metrics(&valid())
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::NotAssessed
    );
}

#[test]
fn missing_ce_marking_is_non_compliant() {
    let mut d = valid();
    d["ceMarking"] = json!(false);
    assert_eq!(
        ToyPlugin.calculate_metrics(&d).unwrap().compliance_status,
        PluginComplianceStatus::NonCompliant
    );
}

#[test]
fn ce_marking_must_be_bool() {
    let mut d = valid();
    d["ceMarking"] = json!("yes");
    assert!(ToyPlugin.validate_input(&d).is_err());
}
