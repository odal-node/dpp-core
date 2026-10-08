//! What the plugin accepts, refuses and reports.

use dpp_plugin_sdk::traits::{DppProductGroupPlugin, PluginComplianceStatus};
use serde_json::{Value, json};

use crate::MattressPlugin;

fn valid() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "primaryMaterial": "upholstered",
        "countryOfOrigin": "SE",
        "co2ePerUnitKg": 48.0,
        "repairabilityScore": 3.0,
        "recycledContentPct": 12.5
    })
}

#[test]
fn valid_input_surfaces_metrics() {
    let r = MattressPlugin.calculate_metrics(&valid()).unwrap();
    assert_eq!(r.co2e_score(), Some(48.0));
    assert_eq!(r.repairability_index(), Some(3.0));
    assert_eq!(r.compliance_status, PluginComplianceStatus::NotAssessed);
}

#[test]
fn only_the_required_fields_are_needed() {
    let minimal = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "primaryMaterial": "mixed",
        "countryOfOrigin": "DE"
    });
    assert!(MattressPlugin.validate_input(&minimal).is_ok());
    let r = MattressPlugin.calculate_metrics(&minimal).unwrap();
    assert_eq!(r.co2e_score(), None);
}

#[test]
fn each_required_field_is_required() {
    for field in ["productIdentifier", "primaryMaterial", "countryOfOrigin"] {
        let mut d = valid();
        d.as_object_mut().unwrap().remove(field);
        assert!(
            MattressPlugin.validate_input(&d).is_err(),
            "{field} is required by the schema and must be by the plugin"
        );
    }
}

/// A v1.0.0 record carries `gtin` where v1.1.0 carries `productIdentifier`. A
/// lens reads it forward before it gets here, so the plugin sees only the new
/// shape and refuses the old one rather than guessing.
#[test]
fn a_gtin_shaped_payload_is_refused() {
    let mut d = valid();
    d.as_object_mut().unwrap().remove("productIdentifier");
    d["gtin"] = json!("12345678901231");
    assert!(MattressPlugin.validate_input(&d).is_err());
}

#[test]
fn invalid_country_fails() {
    let mut d = valid();
    d["countryOfOrigin"] = json!("Sweden");
    assert!(MattressPlugin.validate_input(&d).is_err());
}

#[test]
fn a_material_the_schema_does_not_offer_is_refused() {
    let mut d = valid();
    d["primaryMaterial"] = json!("latex-foam");
    assert!(MattressPlugin.validate_input(&d).is_err());
}

#[test]
fn out_of_range_metrics_are_rejected() {
    for (field, bad) in [
        ("co2ePerUnitKg", -999.0),
        ("repairabilityScore", -1.0),
        ("repairabilityScore", 10.5),
        ("recycledContentPct", 101.0),
    ] {
        let mut d = valid();
        d[field] = json!(bad);
        assert!(
            MattressPlugin.validate_input(&d).is_err(),
            "{field} = {bad} must be rejected"
        );
    }
}

#[test]
fn generate_passport_returns_the_validated_input_and_refuses_invalid_input() {
    let d = valid();
    assert_eq!(MattressPlugin.generate_passport(d.clone()).unwrap(), d);

    let mut bad = valid();
    bad["countryOfOrigin"] = json!("xx");
    assert!(MattressPlugin.generate_passport(bad).is_err());
}

#[test]
fn the_plugin_names_the_group_it_serves_and_reports_the_artifact_version() {
    let identity = MattressPlugin.plugin_identity();
    assert_eq!(identity.product_group, "mattress");
    assert_eq!(identity.version, env!("CARGO_PKG_VERSION"));
}
