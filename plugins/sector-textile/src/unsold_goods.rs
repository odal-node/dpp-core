//! Unsold Goods Destruction Ban — EU ESPR Article 25 / Annex VII, effective 2026-07-19.
//!
//! - `exempt_destruction` with justification < 10 chars → NON_COMPLIANT.
//! - `exempt_destruction` with justification ≥ 10 chars → COMPLIANT (flagged).
//! - Approved destinations (donation/recycling/repurposing/supplier_return) → COMPLIANT.
//! - Anything else → NON_COMPLIANT.

use dpp_plugin_sdk::traits::{PluginComplianceStatus, PluginResult};
use dpp_plugin_sdk::validate::{num, str_of};
use serde_json::{json, Value};

pub fn calculate(input: &Value) -> PluginResult {
    let destination = str_of(input, "destination").unwrap_or("");
    let volume_kg = num(input, "volumeKg");

    let (status, detail): (PluginComplianceStatus, &str) = match destination {
        "exempt_destruction" => {
            let justification = str_of(input, "destructionJustification").unwrap_or("");
            if justification.len() < 10 {
                (
                    PluginComplianceStatus::NonCompliant,
                    "exempt_destruction requires destructionJustification of at least 10 characters",
                )
            } else {
                (PluginComplianceStatus::Compliant, "exempt destruction with valid justification")
            }
        }
        "donation" | "recycling" | "repurposing" | "supplier_return" => {
            (PluginComplianceStatus::Compliant, "approved disposal destination")
        }
        "" => (PluginComplianceStatus::NonCompliant, "missing destination field"),
        _ => (PluginComplianceStatus::NonCompliant, "unknown destination"),
    };

    PluginResult::new(status).with_extra(json!({
        "regulationArticle": "ESPR Article 25 / Annex VII",
        "effectiveDate": "2026-07-19",
        "destination": destination,
        "detail": detail,
        "volumeKg": volume_kg,
    }))
}
