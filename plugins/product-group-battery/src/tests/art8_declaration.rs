//! The Art. 8(1) reporting year, and the Art. 8(4) second-life carve-out.

use dpp_plugin_sdk::traits::DppProductGroupPlugin;
use serde_json::{Value, json};

use crate::BatteryPlugin;

fn shares_without_year() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "industrial",
        "batteryChemistry": "NMC",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "placedOnMarketDate": "2032-01-01",
        "recycledContentCobaltPct": 20.0
    })
}

#[test]
fn shares_without_a_reporting_year_are_flagged() {
    let result = BatteryPlugin
        .calculate_metrics(&shares_without_year())
        .unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.code == "battery.recycled_content.reporting_year_missing"),
        "got: {:?}",
        result.warnings
    );
}

#[test]
fn shares_with_a_reporting_year_are_not_flagged() {
    let mut data = shares_without_year();
    data["recycledContentReportingYear"] = json!(2032);
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.code == "battery.recycled_content.reporting_year_missing"),
        "got: {:?}",
        result.warnings
    );
}

// ── Art. 8(4), read off `batteryStatus` ──────────────────────────────────
//
// The rule itself is tested in `dpp-rules`. What is tested here is the
// wiring: that this plugin reads the carve-out off the payload at all, that
// the closed set is read correctly, and that an exemption is *stated* rather
// than left as an absence a reader cannot distinguish from silence.

/// An EV battery in Phase 1 declaring cobalt far under the 16 % minimum.
/// Without a second-life status this must produce a shortfall.
fn ev_short_on_cobalt() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "ev",
        "batteryChemistry": "NMC",
        "nominalVoltageV": 400.0,
        "nominalCapacityAh": 200.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "placedOnMarketDate": "2032-01-01",
        "recycledContentReportingYear": 2032,
        "recycledContentCobaltPct": 5.0
    })
}

#[test]
fn without_a_second_life_status_the_shortfall_is_reported() {
    let result = BatteryPlugin
        .calculate_metrics(&ev_short_on_cobalt())
        .unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.code == "battery.recycled_content.cobalt_below_2031_target"),
        "the control case must report a shortfall, or the exemption tests \
         below prove nothing; got: {:?}",
        result.warnings
    );
}

#[test]
fn a_remanufactured_battery_is_excused_and_told_so() {
    let mut data = ev_short_on_cobalt();
    data["batteryStatus"] = json!("remanufactured");
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.code == "battery.recycled_content.cobalt_below_2031_target"),
        "Art. 8(4) removes the duty, so no shortfall may be reported; got: {:?}",
        result.warnings
    );
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.code == "battery.recycled_content.exempt_second_life"),
        "the exemption must be stated, not left as silence; got: {:?}",
        result.warnings
    );
}

#[test]
fn waste_is_not_a_second_life_operation() {
    // Annex XIII point 4(c) closed set: original | repurposed | re-used |
    // remanufactured | waste. Only the middle three are Art. 8(4)
    // operations — end-of-life is not preparation for re-use, and a waste
    // battery is not being placed on the market.
    for status in ["waste", "original"] {
        let mut data = ev_short_on_cobalt();
        data["batteryStatus"] = json!(status);
        let result = BatteryPlugin.calculate_metrics(&data).unwrap();
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.code == "battery.recycled_content.cobalt_below_2031_target"),
            "'{status}' is not an Art. 8(4) operation; got: {:?}",
            result.warnings
        );
    }
}

#[test]
fn an_unreadable_status_does_not_excuse_the_duty() {
    // The schema already constrains `batteryStatus` to the five lowercase
    // Annex XIII values, so none of these reaches a validated passport.
    // Pinned anyway: a plugin must not read as exempt anything the schema
    // would not admit, and wrongly excusing a duty is the error an
    // authority finds later rather than one the operator can correct.
    for status in [json!("REMANUFACTURED"), json!(""), json!(7)] {
        let mut data = ev_short_on_cobalt();
        data["batteryStatus"] = status.clone();
        let result = BatteryPlugin.calculate_metrics(&data).unwrap();
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.code == "battery.recycled_content.cobalt_below_2031_target"),
            "batteryStatus {status:?} must not read as exempt; got: {:?}",
            result.warnings
        );
    }
}

#[test]
fn no_shares_declared_means_no_reporting_year_finding() {
    // The duty attaches to declaring a share, not to existing.
    let mut data = shares_without_year();
    data.as_object_mut()
        .unwrap()
        .remove("recycledContentCobaltPct");
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.code == "battery.recycled_content.reporting_year_missing"),
        "got: {:?}",
        result.warnings
    );
}
