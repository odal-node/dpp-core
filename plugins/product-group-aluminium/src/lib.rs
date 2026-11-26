//! Aluminium product group plugin — EU ESPR, CBAM-aligned carbon intensity.
//!
//! Thresholds (kg CO₂e per tonne) are production-route dependent:
//! primary (Hall-Héroult) ≤ 10 000, secondary-recycled ≤ 1 000, mixed ≤ 5 000.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppProductGroupPlugin, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, PluginError,
    PluginIdentity, PluginInput, PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, num, str_of, threshold_status};
use serde_json::{Value, json};

#[derive(Default)]
struct AluminiumPlugin;

impl DppProductGroupPlugin for AluminiumPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            product_group: "aluminium",
            name: "Odal Node Aluminium Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU ESPR aluminium carbon-intensity validation",
        }
    }

    fn schema_version_range(&self) -> SchemaVersionRange {
        SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "1.2.0".into(),
        }
    }

    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> {
        Validator::new(input)
            .require_product_identifier("productIdentifier")
            .require_str("alloyGrade")
            .require_enum(
                "productionRoute",
                &["primary", "secondary-recycled", "mixed"],
            )
            .require_non_negative("co2ePerTonneKg")
            .require_pct("recycledContentPct")
            .require_country("countryOfOrigin")
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        let co2e_kg = num(input, "co2ePerTonneKg");
        let recycled = num(input, "recycledContentPct");
        let route = str_of(input, "productionRoute").unwrap_or("");
        let threshold_kg = match route {
            "primary" => 10_000.0,
            "secondary-recycled" => 1_000.0,
            "mixed" => 5_000.0,
            // Unreachable after validate_input rejects unknown routes; fail
            // closed on the strictest threshold rather than the most permissive.
            _ => 1_000.0,
        };
        let status = threshold_status(co2e_kg, threshold_kg);
        Ok(PluginResult::new(status)
            .maybe_metric(METRIC_CO2E_SCORE, co2e_kg)
            .maybe_metric(METRIC_RECYCLED_CONTENT_PCT, recycled)
            .with_extra(json!({
                "productionRoute": route,
                "thresholdKgCo2ePerTonne": threshold_kg,
            })))
    }

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        self.validate_input(&input)?;
        Ok(input)
    }
}

export_plugin!(AluminiumPlugin);

#[cfg(test)]
mod tests;
