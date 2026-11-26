//! A version and a record the lens tests share.

use semver::Version;
use serde_json::Value;

pub(super) fn v(s: &str) -> Version {
    s.parse().unwrap()
}

/// A minimal but valid v1 battery record (schema-required fields), plus a
/// rated capacity so the lens has something to derive.
pub(super) fn battery_v1() -> Value {
    serde_json::json!({
        "gtin": "09506000134352",
        "batteryChemistry": "LFP",
        "nominalVoltageV": 48.0,
        "nominalCapacityAh": 100.0,
        "expectedLifetimeCycles": 3000,
        "co2ePerUnitKg": 45.2,
        "ratedCapacityKwh": 4.8
    })
}
