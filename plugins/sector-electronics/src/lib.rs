//! Electronics sector plugin — EU Electronics DPP (adopted 18 March 2026).
//!
//! Compliance rules:
//! - Energy class E/F/G → NON_COMPLIANT (fails minimum ecodesign requirement).
//! - Repairability score < 4.0 → NON_COMPLIANT (below EU minimum).
//! - Repairability ≥ 6.0 AND energy class A/B/C → COMPLIANT.
//! - Otherwise (borderline) → NOT_ASSESSED.
//!
//! "electronics" is the **sector**; `productCategory` (`smartphone`, `laptop`,
//! …) is a product category the plugin records but does not dispatch on.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppSectorPlugin, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, METRIC_REPAIRABILITY_INDEX,
    PluginComplianceStatus, PluginError, PluginIdentity, PluginInput, PluginResult,
    SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, num, str_of};
use serde_json::Value;

#[derive(Default)]
struct ElectronicsPlugin;

impl DppSectorPlugin for ElectronicsPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            sector: "electronics",
            name: "Odal Node Electronics Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU Electronics DPP energy/repairability validation",
        }
    }

    fn schema_version_range(&self) -> SchemaVersionRange {
        SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "1.0.0".into(),
        }
    }

    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> {
        Validator::new(input)
            .require_gtin("gtin")
            .require_str("productCategory")
            .require_enum(
                "energyEfficiencyClass",
                &["A", "B", "C", "D", "E", "F", "G"],
            )
            .require_non_negative("co2ePerUnitKg")
            .optional_pct("recycledContentPct")
            // Optional, but when present it gates the verdict (pass ≥ 6.0), so it
            // must be a real 0–10 score, not an unbounded value.
            .optional_range("repairabilityScore", 0.0, 10.0)
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        let co2e = num(input, "co2ePerUnitKg");
        let repair = num(input, "repairabilityScore");
        let recycled = num(input, "recycledContentPct");
        let energy_class = str_of(input, "energyEfficiencyClass");

        let energy_fail = matches!(energy_class, Some("E") | Some("F") | Some("G"));
        let energy_pass = matches!(energy_class, Some("A") | Some("B") | Some("C"));
        let repair_fail = repair.is_some_and(|s| s < 4.0);
        let repair_pass = repair.is_some_and(|s| s >= 6.0);

        let status = if energy_fail || repair_fail {
            PluginComplianceStatus::NonCompliant
        } else if energy_pass && repair_pass {
            PluginComplianceStatus::Compliant
        } else {
            PluginComplianceStatus::NotAssessed
        };

        Ok(PluginResult::new(status)
            .maybe_metric(METRIC_CO2E_SCORE, co2e)
            .maybe_metric(METRIC_REPAIRABILITY_INDEX, repair)
            .maybe_metric(METRIC_RECYCLED_CONTENT_PCT, recycled))
    }

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        self.validate_input(&input)?;
        Ok(input)
    }
}

export_plugin!(ElectronicsPlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base() -> Value {
        json!({
            "gtin": "12345678901231",
            "productCategory": "smartphone",
            "energyEfficiencyClass": "A",
            "co2ePerUnitKg": 55.0
        })
    }

    #[test]
    fn good_energy_and_repair_is_compliant() {
        let mut d = base();
        d["repairabilityScore"] = json!(7.5);
        assert_eq!(
            ElectronicsPlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::Compliant
        );
    }

    #[test]
    fn bad_energy_class_is_non_compliant() {
        let mut d = base();
        d["energyEfficiencyClass"] = json!("F");
        assert_eq!(
            ElectronicsPlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn missing_repair_is_not_assessed() {
        assert_eq!(
            ElectronicsPlugin
                .calculate_metrics(&base())
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NotAssessed
        );
    }

    #[test]
    fn invalid_energy_class_fails_validation() {
        let mut d = base();
        d["energyEfficiencyClass"] = json!("Z");
        assert!(ElectronicsPlugin.validate_input(&d).is_err());
    }

    #[test]
    fn out_of_range_repairability_fails_validation() {
        // An unbounded score must not sail through and force a Compliant verdict.
        let mut d = base();
        d["repairabilityScore"] = json!(999999.0);
        assert!(ElectronicsPlugin.validate_input(&d).is_err());
    }
}
