//! Tests of the `product-group-electronics` module: validation.

use super::super::*;
use serde_json::json;

fn base() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "productCategory": "smartphone",
        "energyEfficiencyClass": "A",
        "co2ePerUnitKg": 55.0
    })
}

#[test]
fn good_energy_and_repair_is_compliant() {
    let mut d = base();
    d["repairabilityScore"] = json!(7.5);
    assert_eq!(
        ElectronicsPlugin
            .calculate_metrics(&d)
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::Compliant
    );
}

#[test]
fn bad_energy_class_is_non_compliant() {
    let mut d = base();
    d["energyEfficiencyClass"] = json!("F");
    assert_eq!(
        ElectronicsPlugin
            .calculate_metrics(&d)
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::NonCompliant
    );
}

#[test]
fn missing_repair_is_not_assessed() {
    assert_eq!(
        ElectronicsPlugin
            .calculate_metrics(&base())
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::NotAssessed
    );
}

#[test]
fn invalid_energy_class_fails_validation() {
    let mut d = base();
    d["energyEfficiencyClass"] = json!("Z");
    assert!(ElectronicsPlugin.validate_input(&d).is_err());
}

#[test]
fn out_of_scope_product_category_fails_validation() {
    // "laptop" is not within Regulation (EU) 2023/1670 Art. 1(1)'s scope.
    let mut d = base();
    d["productCategory"] = json!("laptop");
    assert!(ElectronicsPlugin.validate_input(&d).is_err());
}

#[test]
fn out_of_range_repairability_fails_validation() {
    // An unbounded score must not sail through and force a Compliant verdict.
    let mut d = base();
    d["repairabilityScore"] = json!(999999.0);
    assert!(ElectronicsPlugin.validate_input(&d).is_err());
}
