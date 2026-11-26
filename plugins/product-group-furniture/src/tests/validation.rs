//! Tests of the `product-group-furniture` module: validation.

use super::super::*;
use serde_json::json;

fn valid() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "productType": "chair",
        "primaryMaterial": "solid-wood",
        "countryOfOrigin": "SE",
        "co2ePerUnitKg": 22.0,
        "repairabilityScore": 8.0
    })
}

#[test]
fn valid_input_surfaces_metrics() {
    let r = FurniturePlugin.calculate_metrics(&valid()).unwrap();
    assert_eq!(r.co2e_score(), Some(22.0));
    assert_eq!(r.repairability_index(), Some(8.0));
    assert_eq!(r.compliance_status, PluginComplianceStatus::NotAssessed);
}

#[test]
fn invalid_country_fails() {
    let mut d = valid();
    d["countryOfOrigin"] = json!("Sweden");
    assert!(FurniturePlugin.validate_input(&d).is_err());
}

#[test]
fn negative_metrics_are_rejected() {
    let mut d = valid();
    d["co2ePerUnitKg"] = json!(-999.0);
    assert!(FurniturePlugin.validate_input(&d).is_err());

    let mut d = valid();
    d["repairabilityScore"] = json!(-1.0);
    assert!(FurniturePlugin.validate_input(&d).is_err());
}
