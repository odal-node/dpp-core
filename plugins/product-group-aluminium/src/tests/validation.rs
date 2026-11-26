//! Tests of the `product-group-aluminium` module: validation.

use super::super::*;
use dpp_plugin_sdk::traits::PluginComplianceStatus;
use serde_json::json;

fn valid() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "alloyGrade": "6xxx",
        "productionRoute": "secondary-recycled",
        "co2ePerTonneKg": 600.0,
        "recycledContentPct": 75.0,
        "countryOfOrigin": "NO"
    })
}

#[test]
fn recycled_route_under_threshold_is_compliant() {
    assert_eq!(
        AluminiumPlugin
            .calculate_metrics(&valid())
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::Compliant
    );
}

#[test]
fn primary_route_over_threshold_non_compliant() {
    let mut d = valid();
    d["productionRoute"] = json!("primary");
    d["co2ePerTonneKg"] = json!(15_000.0);
    assert_eq!(
        AluminiumPlugin
            .calculate_metrics(&d)
            .unwrap()
            .compliance_status,
        PluginComplianceStatus::NonCompliant
    );
}

#[test]
fn out_of_range_recycled_pct_fails() {
    let mut d = valid();
    d["recycledContentPct"] = json!(120.0);
    assert!(AluminiumPlugin.validate_input(&d).is_err());
}

#[test]
fn unrecognized_production_route_fails_validation() {
    // Wrong case / unlisted route must be rejected, not fall through to the
    // most-permissive threshold.
    for route in ["Primary", "electric_arc", "unknown"] {
        let mut d = valid();
        d["productionRoute"] = json!(route);
        assert!(
            AluminiumPlugin.validate_input(&d).is_err(),
            "route {route:?} should fail validation"
        );
    }
}
