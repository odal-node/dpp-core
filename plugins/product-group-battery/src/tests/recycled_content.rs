//! Art. 8 recycled-content findings: which phase binds a battery, which metals
//! its chemistry makes relevant, and what a shortfall says.

use dpp_plugin_sdk::traits::{DppProductGroupPlugin, PluginComplianceStatus};
use serde_json::json;

use crate::BatteryPlugin;

#[test]
fn nmc_below_2031_cobalt_emits_advisory_warning_not_violation() {
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "industrial",
        "batteryChemistry": "NMC",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "placedOnMarketDate": "2032-01-01", // Phase 1 binds
        "recycledContentCobaltPct": 12.0,   // < 16 Phase-1 target
        "recycledContentNickelPct": 8.0,    // >= 6 ok
        "recycledContentLithiumPct": 6.0    // >= 6 ok
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert_eq!(
        result.compliance_status,
        PluginComplianceStatus::NotAssessed
    );
    // Advisory only, never blocking — the plugin does not gate on Art. 8.
    assert!(result.violations.is_empty());
    let cobalt = result
        .warnings
        .iter()
        .find(|w| w.code.contains("cobalt"))
        .unwrap_or_else(|| panic!("expected cobalt advisory, got: {:?}", result.warnings));
    assert!(
        cobalt.message.contains("binding for this battery"),
        "a battery placed on the market in 2032 is bound by Art. 8(2); got: {}",
        cobalt.message
    );
    assert!(!result.warnings.iter().any(|w| w.code.contains("nickel")));
    assert!(!result.warnings.iter().any(|w| w.code.contains("lithium")));
}

#[test]
fn lfp_zero_cobalt_does_not_warn() {
    // LFP contains no cobalt; a defaulted 0.0 must not produce a shortfall.
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "industrial",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 45.2,
        "placedOnMarketDate": "2032-01-01",
        "recycledContentReportingYear": 2032,
        "recycledContentCobaltPct": 0.0,
        "recycledContentLithiumPct": 12.5
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    // 🚨 Narrowed from `warnings.is_empty()`. That held only while this
    // record carried no `batteryType`: an unknown category makes the Annex
    // XIII mandatory-content check answer `Unknown` for every field and
    // emit nothing, so a blanket assertion was silently resting on a
    // determination that was not being made. This is a cobalt-on-LFP test;
    // it now asserts about cobalt on LFP.
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.code.contains("not_in_chemistry")),
        "LFP cobalt 0.0 must not warn; got: {:?}",
        result.warnings
    );
}

#[test]
fn portable_battery_skips_recycled_content_warnings() {
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryChemistry": "NMC",
        "batteryType": "portable",
        "nominalVoltageV": 3.6,
        "nominalCapacityAh": 2.0,
        "expectedLifetimeCycles": 500,
        "co2ePerUnitKg": 5.0,
        "recycledContentCobaltPct": 0.0
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        result.warnings.is_empty(),
        "portable batteries are out of Art. 8(2) Phase-1 scope; got: {:?}",
        result.warnings
    );
}
#[test]
fn lmt_battery_below_target_gets_no_phase1_advisory() {
    // LMT (e-bike/e-scooter) batteries are out of Phase-1 scope (Phase 2
    // only, 2036), so a below-2031-target declaration must not be flagged.
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryChemistry": "NMC",
        "batteryType": "lmt",
        "nominalVoltageV": 36.0,
        "nominalCapacityAh": 10.0,
        "expectedLifetimeCycles": 800,
        "co2ePerUnitKg": 12.0,
        "placedOnMarketDate": "2032-01-01",
        "recycledContentCobaltPct": 5.0 // < 16, but LMT is out of Phase 1
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.code.contains("below_2031")),
        "LMT must not get a Phase-1 shortfall advisory; got: {:?}",
        result.warnings
    );
}

#[test]
fn battery_placed_before_2031_gets_no_binding_shortfall() {
    // The reason placedOnMarketDate exists. This battery declares 12 %
    // cobalt — below the Art. 8(2) minimum of 16 % — but was placed on the
    // EU market in 2030, so that minimum never attaches to it. Deriving the
    // phase from "today" would report it as short from 18 Aug 2031 onwards.
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "industrial",
        "batteryChemistry": "NMC",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "placedOnMarketDate": "2030-06-01",
        "recycledContentCobaltPct": 12.0
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    let cobalt = result
        .warnings
        .iter()
        .find(|w| w.code.contains("cobalt"))
        .unwrap_or_else(|| {
            panic!(
                "expected forward-looking advisory, got: {:?}",
                result.warnings
            )
        });
    assert!(
        cobalt.message.contains("not binding for this battery"),
        "a battery placed on the market in 2030 is outside Art. 8(2); got: {}",
        cobalt.message
    );
    assert!(result.violations.is_empty());
}

#[test]
fn missing_market_date_is_reported_rather_than_guessed() {
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "industrial",
        "batteryChemistry": "NMC",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "recycledContentCobaltPct": 12.0
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.code == "battery.recycled_content.market_date_missing"),
        "got: {:?}",
        result.warnings
    );
    // No phase can be determined, so no shortfall may be asserted either way.
    assert!(
        !result.warnings.iter().any(|w| w.code.contains("below_")),
        "no shortfall without a date; got: {:?}",
        result.warnings
    );
}

#[test]
fn malformed_market_date_is_treated_as_missing() {
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "industrial",
        "batteryChemistry": "NMC",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "placedOnMarketDate": "01/06/2030",
        "recycledContentCobaltPct": 12.0
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.code == "battery.recycled_content.market_date_missing"),
        "an unparseable date must not silently become a determination; got: {:?}",
        result.warnings
    );
}

#[test]
fn lmt_placed_after_2036_is_bound_by_phase_two() {
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryChemistry": "NMC",
        "batteryType": "lmt",
        "nominalVoltageV": 36.0,
        "nominalCapacityAh": 10.0,
        "expectedLifetimeCycles": 800,
        "co2ePerUnitKg": 12.0,
        "placedOnMarketDate": "2037-01-01",
        "recycledContentCobaltPct": 20.0 // clears 2031's 16 %, short of 2036's 26 %
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    let cobalt = result
        .warnings
        .iter()
        .find(|w| w.code.contains("cobalt"))
        .unwrap_or_else(|| panic!("expected a Phase-2 advisory, got: {:?}", result.warnings));
    assert!(cobalt.code.contains("below_2036"), "got: {}", cobalt.code);
    assert!(
        cobalt.message.contains("binding for this battery"),
        "got: {}",
        cobalt.message
    );
}

#[test]
fn small_industrial_below_target_gets_no_phase1_advisory() {
    // Industrial batteries ≤ 2 kWh are out of Phase-1 scope.
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryChemistry": "NMC",
        "batteryType": "industrial",
        "nominalVoltageV": 12.0,
        "nominalCapacityAh": 100.0, // 1.2 kWh ≤ 2 kWh
        "expectedLifetimeCycles": 1000,
        "co2ePerUnitKg": 20.0,
        "recycledContentCobaltPct": 5.0 // < 16, but ≤ 2 kWh is out of scope
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.code.contains("below_2031")),
        "≤2 kWh industrial must not get a Phase-1 shortfall advisory; got: {:?}",
        result.warnings
    );
}

#[test]
fn large_industrial_below_target_gets_phase1_advisory() {
    // > 2 kWh industrial IS in Phase-1 scope.
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryChemistry": "NMC",
        "batteryType": "industrial",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0, // 4.8 kWh > 2 kWh
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "placedOnMarketDate": "2032-01-01",
        "recycledContentCobaltPct": 5.0 // < 16
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.code.contains("cobalt_below_2031")),
        "in-scope industrial shortfall must be flagged; got: {:?}",
        result.warnings
    );
}

#[test]
fn cobalt_declared_on_lfp_emits_chemistry_conflict_advisory() {
    // LFP contains no cobalt; a positive cobalt declaration is a data
    // contradiction that must be surfaced, not silently accepted.
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "industrial",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 45.2,
        "recycledContentCobaltPct": 8.0,
        "recycledContentLithiumPct": 12.0
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.code == "battery.recycled_content.cobalt_not_in_chemistry"),
        "cobalt-on-LFP must emit a chemistry-conflict advisory; got: {:?}",
        result.warnings
    );
}
