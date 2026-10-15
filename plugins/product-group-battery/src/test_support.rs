//! A battery record the plugin's tests start from.

use serde_json::{Value, json};

/// A structurally valid industrial LFP battery.
pub(crate) fn valid_battery() -> Value {
    json!({
        "productIdentifier": {"scheme": "gs1", "gtin": "12345678901231"},
        // 🚨 The schema has required this since v1.0.0 and this
        // fixture omitted it, so every test built on it ran against
        // a record the schema would refuse — a large part of how the
        // missing check stayed invisible.
        "batteryType": "industrial",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 85.4,
        "recycledContentCobaltPct": 16.0,
        "recycledContentLithiumPct": 6.0
    })
}
