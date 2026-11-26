//! Tests of the `product-group-tyre` module: validation.

use super::super::*;
use serde_json::json;

fn valid() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "tyreClass": "C1",
        "fuelEfficiencyClass": "B",
        "wetGripClass": "A",
        "externalRollingNoiseDb": 70.0,
        "recycledRubberPct": 15.0
    })
}

#[test]
fn valid_input_passes_and_surfaces_rubber() {
    let r = TyrePlugin.calculate_metrics(&valid()).unwrap();
    assert_eq!(r.recycled_content_pct(), Some(15.0));
    assert_eq!(r.compliance_status, PluginComplianceStatus::NotAssessed);
}

#[test]
fn old_a_to_g_grip_scale_is_rejected() {
    let mut d = valid();
    d["wetGripClass"] = json!("F"); // 2020/740 is A–E only
    assert!(TyrePlugin.validate_input(&d).is_err());
}

#[test]
fn invalid_tyre_class_is_rejected() {
    let mut d = valid();
    d["tyreClass"] = json!("garbage"); // must be C1/C2/C3
    assert!(TyrePlugin.validate_input(&d).is_err());
}

#[test]
fn negative_co2e_per_tyre_is_rejected() {
    let mut d = valid();
    d["co2ePerTyreKg"] = json!(-50.0);
    assert!(TyrePlugin.validate_input(&d).is_err());
}
