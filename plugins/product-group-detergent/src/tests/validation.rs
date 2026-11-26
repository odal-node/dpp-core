//! Tests of the `product-group-detergent` module: validation.

use super::super::*;
use serde_json::json;

fn valid() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "productType": "laundry",
        "format": "liquid",
        "surfactants": [
            { "name": "LAS", "biodegradable": true, "concentrationBand": "5-15%" }
        ],
        "countryOfOrigin": "DE",
        "co2ePerUnitKg": 1.2
    })
}

#[test]
fn all_biodegradable_is_not_assessed() {
    assert_eq!(
        DetergentPlugin
            .calculate_metrics(&valid())
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::NotAssessed
    );
}

#[test]
fn non_biodegradable_surfactant_is_non_compliant() {
    let mut d = valid();
    d["surfactants"][0]["biodegradable"] = json!(false);
    assert_eq!(
        DetergentPlugin
            .calculate_metrics(&d)
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::NonCompliant
    );
}

#[test]
fn empty_surfactants_fails_validation() {
    let mut d = valid();
    d["surfactants"] = json!([]);
    assert!(DetergentPlugin.validate_input(&d).is_err());
}

#[test]
fn undeclared_biodegradable_fails_validation() {
    // biodegradable omitted must be rejected, not silently NotAssessed.
    let mut d = valid();
    d["surfactants"] = json!([{ "name": "LAS", "concentrationBand": "5-15%" }]);
    assert!(DetergentPlugin.validate_input(&d).is_err());
}

#[test]
fn negative_co2e_fails_validation() {
    let mut d = valid();
    d["co2ePerUnitKg"] = json!(-999.0);
    assert!(DetergentPlugin.validate_input(&d).is_err());
}
