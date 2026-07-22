//! Tyre sector plugin — EU Regulation 2020/740 labelling + ESPR Working Plan.
//!
//! Validates the mandatory label fields (fuel-efficiency and wet-grip classes
//! use the A–E scale per 2020/740). DPP mandate and thresholds are expected
//! ~2029, so the determination is `NOT_ASSESSED`. `tyreClass` (`C1`/`C2`/`C3`)
//! is a product category, not a dispatch key.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppSectorPlugin, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, PluginComplianceStatus,
    PluginError, PluginIdentity, PluginInput, PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, num};
use serde_json::Value;

#[derive(Default)]
struct TyrePlugin;

impl DppSectorPlugin for TyrePlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            sector: "tyre",
            name: "Odal Node Tyre Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU 2020/740 tyre labelling validation",
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
            .require_enum("tyreClass", &["C1", "C2", "C3"])
            .require_enum("fuelEfficiencyClass", &["A", "B", "C", "D", "E"])
            .require_enum("wetGripClass", &["A", "B", "C", "D", "E"])
            .require_non_negative("externalRollingNoiseDb")
            .optional_pct("recycledRubberPct")
            .optional_non_negative("co2ePerTyreKg")
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        Ok(PluginResult::new(PluginComplianceStatus::NotAssessed)
            .maybe_metric(METRIC_CO2E_SCORE, num(input, "co2ePerTyreKg"))
            // For tyres, "recycled content" is recycled rubber.
            .maybe_metric(METRIC_RECYCLED_CONTENT_PCT, num(input, "recycledRubberPct")))
    }

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        self.validate_input(&input)?;
        Ok(input)
    }
}

export_plugin!(TyrePlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({
            "gtin": "12345678901231",
            "tyreClass": "C1",
            "fuelEfficiencyClass": "B",
            "wetGripClass": "A",
            "externalRollingNoiseDb": 70.0,
            "recycledRubberPct": 15.0
        })
    }

    #[test]
    fn valid_input_passes_and_surfaces_rubber() {
        let r = TyrePlugin.calculate_metrics(&valid()).unwrap();
        assert_eq!(r.recycled_content_pct(), Some(15.0));
        assert_eq!(r.compliance_status, PluginComplianceStatus::NotAssessed);
    }

    #[test]
    fn old_a_to_g_grip_scale_is_rejected() {
        let mut d = valid();
        d["wetGripClass"] = json!("F"); // 2020/740 is A–E only
        assert!(TyrePlugin.validate_input(&d).is_err());
    }

    #[test]
    fn invalid_tyre_class_is_rejected() {
        let mut d = valid();
        d["tyreClass"] = json!("garbage"); // must be C1/C2/C3
        assert!(TyrePlugin.validate_input(&d).is_err());
    }

    #[test]
    fn negative_co2e_per_tyre_is_rejected() {
        let mut d = valid();
        d["co2ePerTyreKg"] = json!(-50.0);
        assert!(TyrePlugin.validate_input(&d).is_err());
    }
}
