//! Tests of the `product-group-construction` module: validation.

use super::super::*;
use serde_json::json;

fn valid() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "productFamily": "cement",
        "countryOfOrigin": "PL",
        "co2ePerFunctionalUnitKg": 780.0,
        "functionalUnit": "per tonne"
    })
}

#[test]
fn valid_input_is_not_assessed_with_co2e() {
    let r = ConstructionPlugin.calculate_metrics(&valid()).unwrap();
    assert_eq!(r.compliance_status, PluginComplianceStatus::NotAssessed);
    assert_eq!(r.co2e_score(), Some(780.0));
}

#[test]
fn missing_functional_unit_fails() {
    let mut d = valid();
    d.as_object_mut().unwrap().remove("functionalUnit");
    assert!(ConstructionPlugin.validate_input(&d).is_err());
}
