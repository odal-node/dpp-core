//! Textile fibre-composition compliance — fibre percentages must sum to ~100%
//! (± 2.0 tolerance), and carbon/recycled/repair metrics pass through.

use dpp_plugin_sdk::traits::{
    METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, METRIC_REPAIRABILITY_INDEX,
    PluginComplianceStatus, PluginResult,
};
use dpp_plugin_sdk::validate::num;
use serde_json::Value;

pub fn calculate(input: &Value) -> PluginResult {
    let status = if fibre_composition_ok(input) {
        PluginComplianceStatus::Compliant
    } else {
        PluginComplianceStatus::NonCompliant
    };
    PluginResult::new(status)
        .maybe_metric(METRIC_CO2E_SCORE, num(input, "carbonFootprintKgCo2e"))
        .maybe_metric(METRIC_REPAIRABILITY_INDEX, num(input, "repairScore"))
        .maybe_metric(
            METRIC_RECYCLED_CONTENT_PCT,
            num(input, "recycledContentPct"),
        )
}

/// Full fibre-composition validity, delegated to the shared `dpp-rules`
/// validator (each `pct` finite and in `[0, 100]`, any `countryOfOrigin` a valid
/// ISO code, and the percentages summing to ~100%). Reimplementing only the sum
/// check here would let out-of-range percentages that happen to sum to 100 —
/// and entries missing a `pct` entirely — pass as compliant.
fn fibre_composition_ok(input: &Value) -> bool {
    let Some(fibres) = input.get("fibreComposition").and_then(Value::as_array) else {
        return false;
    };
    let mut inputs = Vec::with_capacity(fibres.len());
    for f in fibres {
        // A missing/non-numeric pct is an incomplete declaration — fail rather
        // than silently dropping the entry.
        let Some(pct) = f.get("pct").and_then(Value::as_f64) else {
            return false;
        };
        inputs.push(dpp_plugin_sdk::rules::FibreInput {
            fibre: f.get("fibre").and_then(Value::as_str).unwrap_or(""),
            pct,
            country_of_origin: f.get("countryOfOrigin").and_then(Value::as_str),
        });
    }
    dpp_plugin_sdk::rules::validate_fibre_composition(&inputs).is_ok()
}
