//! Annex XIII content per battery category, and the Annex VII state-of-health
//! parameter set.

use dpp_plugin_sdk::traits::{DppProductGroupPlugin, PluginResult};
use serde_json::json;

use crate::BatteryPlugin;
use crate::test_support::valid_battery;

// ── Annex XIII per-category content ──────────────────────────────────────

fn codes(result: &PluginResult) -> Vec<&str> {
    result.warnings.iter().map(|w| w.code.as_str()).collect()
}

/// A category the guidance covers gets told what it still owes — once.
#[test]
fn a_covered_category_is_warned_about_missing_mandatory_content() {
    let mut data = valid_battery();
    data["batteryType"] = json!("ev");
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();

    let hits = codes(&result)
        .into_iter()
        .filter(|c| *c == "battery.annex_xiii.mandatory_content_incomplete")
        .count();
    assert_eq!(
        hits,
        1,
        "one summary finding, not one per field: {:?}",
        codes(&result)
    );

    let msg = &result
        .warnings
        .iter()
        .find(|w| w.code == "battery.annex_xiii.mandatory_content_incomplete")
        .unwrap()
        .message;
    assert!(
        msg.contains("batteryModelId") && msg.contains("manufacturingDate"),
        "the finding must name what is missing: {msg}"
    );
    // The Art. 77(3) identifier is the passport's carrier identifier, not a
    // payload field, so the superseded one is not owed.
    assert!(
        !msg.contains("batteryPassportNumber"),
        "a superseded field is not mandatory content: {msg}"
    );
}

/// The guidance covers EV, LMT and industrial batteries and says nothing
/// about portable or SLI ones. Silence is not a requirement to invent.
#[test]
fn an_uncovered_category_is_not_warned() {
    let mut data = valid_battery();
    data["batteryType"] = json!("portable");
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        !codes(&result).contains(&"battery.annex_xiii.mandatory_content_incomplete"),
        "got: {:?}",
        codes(&result)
    );
}

/// The half of the table a completeness check can never see: a field the
/// guidance says must not be filled, that has been.
#[test]
fn a_field_the_guidance_bars_is_flagged_when_present() {
    let mut data = valid_battery();
    data["batteryType"] = json!("lmt");
    data["carbonFootprintClass"] = json!("B");
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        codes(&result).contains(&"battery.annex_xiii.carbonFootprintClass_not_applicable"),
        "got: {:?}",
        codes(&result)
    );

    // Absent, it is not mentioned — the finding is about the value, not the key.
    data.as_object_mut().unwrap().remove("carbonFootprintClass");
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        !codes(&result).contains(&"battery.annex_xiii.carbonFootprintClass_not_applicable"),
        "got: {:?}",
        codes(&result)
    );
}

// ── Annex VII state of health ────────────────────────────────────────────

/// Part A's two lists are disjoint, and an EV battery reports the one-item
/// one. Declaring the five-parameter list is the wrong set, not a gap.
#[test]
fn an_ev_battery_declaring_the_stationary_parameter_set_is_flagged() {
    let mut data = valid_battery();
    data["batteryType"] = json!("ev");
    data["stateOfHealth"] = json!({
        "parameterSet": "stationaryOrLmt",
        "remainingCapacityPct": 88.0,
        "selfDischargeRatePctPerMonth": 1.5
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        codes(&result).contains(&"battery.annex_vii.wrong_state_of_health_parameter_set"),
        "got: {:?}",
        codes(&result)
    );

    data["stateOfHealth"] = json!({ "parameterSet": "electricVehicle", "socePct": 97.0 });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        !codes(&result).contains(&"battery.annex_vii.wrong_state_of_health_parameter_set"),
        "the set Annex VII Part A actually asks of an EV battery: {:?}",
        codes(&result)
    );
}

/// Part A does not reach portable or SLI batteries, so there is no set to
/// be wrong about.
#[test]
fn a_battery_outside_annex_vii_part_a_is_not_flagged() {
    let mut data = valid_battery();
    data["batteryType"] = json!("portable");
    data["stateOfHealth"] = json!({ "parameterSet": "electricVehicle", "socePct": 97.0 });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        !codes(&result).contains(&"battery.annex_vii.wrong_state_of_health_parameter_set"),
        "got: {:?}",
        codes(&result)
    );
}
