//! Toy product group plugin — EU Delegated Regulation 2025/2509 + Toy Safety Directive.
//!
//! The one hard check available today is CE marking: a toy declaring
//! `ceMarking: false` is `NON_COMPLIANT`. Full safety/DPP thresholds (mandate
//! 2030) are pending, so otherwise the determination is `NOT_ASSESSED`.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppProductGroupPlugin, PluginComplianceStatus, PluginError, PluginIdentity, PluginInput,
    PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::Validator;
use serde_json::Value;

#[derive(Default)]
struct ToyPlugin;

impl DppProductGroupPlugin for ToyPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            product_group: "toy",
            name: "Odal Node Toy Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU 2025/2509 toy safety and CE-marking validation",
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
            .require_str("ageGroup")
            .require_str("primaryMaterial")
            .require_bool("ceMarking")
            .require_country("countryOfOrigin")
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        let ce = input.get("ceMarking").and_then(Value::as_bool);
        let status = if ce == Some(false) {
            PluginComplianceStatus::NonCompliant
        } else {
            PluginComplianceStatus::NotAssessed
        };
        Ok(PluginResult::new(status))
    }

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        self.validate_input(&input)?;
        Ok(input)
    }
}

export_plugin!(ToyPlugin);

#[cfg(test)]
mod tests;
