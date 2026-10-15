//! What the plugin accepts and refuses, and what it says about itself.

use dpp_plugin_sdk::rules::batteries::category::BATTERY_TYPES;
use dpp_plugin_sdk::traits::{AbiVersion, DppProductGroupPlugin, PluginCapability, PluginError};
use serde_json::json;

use crate::BatteryPlugin;
use crate::test_support::valid_battery;

#[test]
fn meta_reports_battery_product_group() {
    assert_eq!(BatteryPlugin.meta().product_group, "battery");
}

#[test]
fn capabilities_cover_battery_schema_range() {
    let caps = BatteryPlugin.capabilities();
    assert_eq!(caps.abi_version, AbiVersion::current());
    assert_eq!(caps.supported_schemas[0].max_version, "2.8.0");
    assert!(caps.capabilities.contains(&PluginCapability::Validate));
}

#[test]
fn valid_input_passes_validation() {
    assert!(BatteryPlugin.validate_input(&valid_battery()).is_ok());
}

#[test]
fn missing_product_identifier_fails_with_field_error() {
    let mut data = valid_battery();
    data.as_object_mut().unwrap().remove("productIdentifier");
    let err = BatteryPlugin.validate_input(&data).unwrap_err();
    match err {
        PluginError::ValidationErrors(errs) => {
            assert!(
                errs.iter()
                    .any(|e| e.field == "/productIdentifier" && e.code == "missing")
            );
        }
        other => panic!("expected ValidationErrors, got {other:?}"),
    }
}

#[test]
fn a_battery_without_a_category_is_refused() {
    // The category decides which obligations apply, so a record without one
    // cannot be assessed against anything. It used to pass this tier and
    // reach `calculate_metrics`, where `unwrap_or("")` turned the absence
    // into a category no rule matches.
    let mut data = valid_battery();
    data.as_object_mut().expect("object").remove("batteryType");
    match BatteryPlugin.validate_input(&data) {
        Err(PluginError::ValidationErrors(errors)) => assert!(
            errors.iter().any(|e| e.field == "/batteryType"),
            "the refusal must name the field: {errors:?}"
        ),
        other => panic!("expected a refusal, got {other:?}"),
    }
}

#[test]
fn a_category_outside_art_1_3_is_refused() {
    // Art. 1(3) is a closed enumeration whose tie-break rule only works
    // over a closed set, so an unrecognised category is a reason to reject
    // the record rather than absorb it.
    let mut data = valid_battery();
    data["batteryType"] = json!("hypothetical");
    assert!(BatteryPlugin.validate_input(&data).is_err());
}

#[test]
fn every_art_1_3_category_is_accepted() {
    // The other direction, so the check cannot be tightened into refusing a
    // lawful category. Driven from the shared vocabulary rather than a list
    // written out here, which is the point of there being one.
    for category in BATTERY_TYPES {
        let mut data = valid_battery();
        data["batteryType"] = json!(category);
        assert!(
            BatteryPlugin.validate_input(&data).is_ok(),
            "{category} is an Art. 1(3) category and must be accepted"
        );
    }
}

#[test]
fn malformed_gtin_fails() {
    let mut data = valid_battery();
    data["productIdentifier"]["gtin"] = json!("12-34");
    let err = BatteryPlugin.validate_input(&data).unwrap_err();
    assert!(matches!(err, PluginError::ValidationErrors(_)));
}

/// 🚨 A battery identified without GS1 is still a battery.
///
/// Scheme 1 is what hid the old defect: while every record carried a bare
/// `gtin`, requiring one always succeeded. Schemes 2 and 3 carry no GTIN
/// at all, so a plugin that asks for one rejects exactly the passports the
/// identifier work introduced.
#[test]
fn a_battery_identified_without_gs1_is_accepted() {
    for identifier in [
        json!({"scheme": "identificationLink", "url": "https://id.acme.example.com/b/1"}),
        json!({"scheme": "did", "did": "did:web:acme.example.com:b:1"}),
    ] {
        let mut data = valid_battery();
        data["productIdentifier"] = identifier.clone();
        assert!(
            BatteryPlugin.validate_input(&data).is_ok(),
            "scheme carrying no GTIN was refused: {identifier}"
        );
    }
}

#[test]
fn non_positive_voltage_fails() {
    let mut data = valid_battery();
    data["nominalVoltageV"] = json!(0.0);
    assert!(BatteryPlugin.validate_input(&data).is_err());
}

/// Schema v2.8.0 no longer requires `co2ePerUnitKg`, so a record without
/// it is valid, and no CO2e metric is invented for it. A value that is
/// present is still held to being non-negative.
#[test]
fn a_record_without_co2e_is_valid_and_reports_no_co2e() {
    let mut data = valid_battery();
    data.as_object_mut().unwrap().remove("co2ePerUnitKg");
    assert!(BatteryPlugin.validate_input(&data).is_ok());
    let result = BatteryPlugin.calculate_metrics(&data).expect("metrics");
    assert_eq!(result.co2e_score(), None);

    data["co2ePerUnitKg"] = json!(-1.0);
    assert!(BatteryPlugin.validate_input(&data).is_err());
}

#[test]
fn out_of_range_recycled_pct_fails() {
    let mut data = valid_battery();
    data["recycledContentCobaltPct"] = json!(140.0);
    let err = BatteryPlugin.validate_input(&data).unwrap_err();
    assert!(matches!(err, PluginError::ValidationErrors(_)));
}
#[test]
fn metrics_reject_invalid_input() {
    let data = json!({ "batteryChemistry": "LFP" });
    assert!(BatteryPlugin.calculate_metrics(&data).is_err());
}

#[test]
fn generate_passport_is_passthrough_on_valid() {
    let data = valid_battery();
    let out = BatteryPlugin.generate_passport(data).unwrap();
    assert_eq!(out["productIdentifier"]["gtin"], "12345678901231");
}
