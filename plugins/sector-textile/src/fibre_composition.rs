//! Textile fibre-composition compliance — fibre percentages must sum to ~100%
//! (± 2.0 tolerance), and carbon/recycled/repair metrics pass through.

use dpp_plugin_sdk::traits::{PluginComplianceStatus, PluginResult, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, METRIC_REPAIRABILITY_INDEX};
use dpp_plugin_sdk::validate::num;
use serde_json::Value;

pub fn calculate(input: &Value) -> PluginResult {
    let status = if fibre_sum_ok(input) {
        PluginComplianceStatus::Compliant
    } else {
        PluginComplianceStatus::NonCompliant
    };
    PluginResult::new(status)
        .maybe_metric(METRIC_CO2E_SCORE, num(input, "carbonFootprintKgCo2e"))
        .maybe_metric(METRIC_REPAIRABILITY_INDEX, num(input, "repairScore"))
        .maybe_metric(METRIC_RECYCLED_CONTENT_PCT, num(input, "recycledContentPct"))
}

fn fibre_sum_ok(input: &Value) -> bool {
    // The fibre-sum rule lives once in `dpp-rules` (shared with dpp-domain).
    match input.get("fibreComposition").and_then(Value::as_array) {
        Some(fibres) if !fibres.is_empty() => {
            let pcts: Vec<f64> = fibres
                .iter()
                .filter_map(|f| f.get("pct").and_then(Value::as_f64))
                .collect();
            dpp_plugin_sdk::rules::fibre_sum_ok(&pcts)
        }
        _ => false,
    }
}
