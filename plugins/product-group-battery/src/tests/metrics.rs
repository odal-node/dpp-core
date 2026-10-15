//! The metrics the plugin reports, and its check that rated capacity agrees
//! with nominal voltage and capacity.

use dpp_plugin_sdk::traits::{DppProductGroupPlugin, PluginComplianceStatus};
use serde_json::json;

use crate::BatteryPlugin;
use crate::test_support::valid_battery;

#[test]
fn metrics_surface_all_declared_metals() {
    let result = BatteryPlugin.calculate_metrics(&valid_battery()).unwrap();
    assert_eq!(result.co2e_score(), Some(85.4));
    assert_eq!(
        result.compliance_status,
        PluginComplianceStatus::NotAssessed
    );
    // Four per-metal minima over two measurement bases do not become one
    // number. The flat metric stays unset; the metals travel under `extra`.
    assert_eq!(
        result.recycled_content_pct(),
        None,
        "a mean of the declared metals is not an Art. 8 figure"
    );
    let extra = result.extra.unwrap();
    assert_eq!(extra["recycledContentByMetal"]["cobaltPct"], 16.0);
    assert_eq!(extra["recycledContentByMetal"]["lithiumPct"], 6.0);
    // Undeclared metals are explicitly null, not dropped.
    assert!(extra["recycledContentByMetal"]["nickelPct"].is_null());
}
#[test]
fn rated_capacity_unit_error_emits_advisory_warning() {
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "industrial",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0, // nominal = 4.8 kWh
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 45.2,
        "ratedCapacityKwh": 48.0 // 10× off → unit error
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    assert!(result.violations.is_empty());
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.code == "battery.rated_capacity.inconsistent_with_nominal"),
        "got: {:?}",
        result.warnings
    );
}

#[test]
fn consistent_rated_capacity_no_warning() {
    let data = json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        "batteryType": "industrial",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 45.2,
        "placedOnMarketDate": "2032-01-01",
        "ratedCapacityKwh": 4.8
    });
    let result = BatteryPlugin.calculate_metrics(&data).unwrap();
    // Narrowed for the same reason as `lfp_zero_cobalt_does_not_warn`: this
    // asserts about the rated-capacity cross-check, not about every rule at once.
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.code == "battery.rated_capacity.inconsistent_with_nominal"),
        "got: {:?}",
        result.warnings
    );
}
