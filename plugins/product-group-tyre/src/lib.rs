//! Tyre product group plugin — EU Regulation 2020/740 labelling + ESPR Working Plan.
//!
//! Validates the mandatory label fields (fuel-efficiency and wet-grip classes
//! use the A–E scale per 2020/740). The DPP mandate and thresholds await a
//! delegated act (the Working Plan's indicative adoption year is 2027; none is
//! adopted), so the determination is `NOT_ASSESSED`. `tyreClass` (`C1`/`C2`/`C3`)
//! is a product category, not a dispatch key.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppProductGroupPlugin, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, PluginComplianceStatus,
    PluginError, PluginIdentity, PluginInput, PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, num};
use serde_json::Value;

#[derive(Default)]
struct TyrePlugin;

impl DppProductGroupPlugin for TyrePlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            product_group: "tyre",
            name: "Odal Node Tyre Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU 2020/740 tyre labelling validation",
        }
    }

    fn schema_version_range(&self) -> SchemaVersionRange {
        SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "1.1.0".into(),
        }
    }

    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> {
        Validator::new(input)
            .require_product_identifier("productIdentifier")
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
mod tests;
