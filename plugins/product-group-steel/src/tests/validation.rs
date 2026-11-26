//! Tests of the `product-group-steel` module: validation.

use super::super::*;
use dpp_plugin_sdk::traits::PluginComplianceStatus;
use serde_json::json;

fn valid() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "co2ePerTonneSteel": 0.35,
        "recycledScrapContentPct": 90.0,
        "productCategory": "long",
        "productionRoute": "electric-arc",
        "countryOfOrigin": "DE"
    })
}

#[test]
fn electric_arc_low_co2e_is_compliant() {
    let r = SteelPlugin.calculate_metrics(&valid()).unwrap();
    assert_eq!(r.compliance_status, PluginComplianceStatus::Compliant);
    assert_eq!(r.co2e_score(), Some(0.35));
}

#[test]
fn blast_furnace_over_threshold_non_compliant() {
    let mut d = valid();
    d["productionRoute"] = json!("blast-furnace");
    d["co2ePerTonneSteel"] = json!(2.9);
    assert_eq!(
        SteelPlugin.calculate_metrics(&d).unwrap().compliance_status,
        PluginComplianceStatus::NonCompliant
    );
}

#[test]
fn missing_country_fails_validation() {
    let mut d = valid();
    d.as_object_mut().unwrap().remove("countryOfOrigin");
    assert!(SteelPlugin.validate_input(&d).is_err());
}

#[test]
fn unrecognized_production_route_fails_validation() {
    // A wrong-case or unlisted route must be rejected, not scored against
    // the most-permissive wildcard threshold.
    for route in ["Electric-Arc", "electric_arc", "unknown"] {
        let mut d = valid();
        d["productionRoute"] = json!(route);
        assert!(
            SteelPlugin.validate_input(&d).is_err(),
            "route {route:?} should fail validation"
        );
    }
}
