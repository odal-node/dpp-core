//! Textile DPP compliance plugin.

mod fibre_composition;

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppProductGroupPlugin, PluginError, PluginIdentity, PluginInput, PluginResult,
    SchemaVersionRange,
};
use dpp_plugin_sdk::validate::Validator;
use serde_json::Value;

#[derive(Default)]
struct TextilePlugin;

impl DppProductGroupPlugin for TextilePlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            product_group: "textile",
            name: "Odal Node Textile Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU textile DPP fibre composition",
        }
    }

    // Declares the textile schema range.
    fn schema_version_range(&self) -> SchemaVersionRange {
        SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "1.4.0".into(),
        }
    }

    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> {
        Validator::new(input)
            // 🚨 Textile never asked for a bare `gtin`, so when every other
            // plugin's `require_gtin` had to be replaced by this call, this
            // one had nothing to replace. It came through the identifier
            // migration clean by accident — and the same accident left it
            // the only plugin checking no identifier at all.
            .require_product_identifier("productIdentifier")
            .require_non_empty_array("fibreComposition")
            .require_country("countryOfOrigin")
            .require_str("careInstructions")
            .require_str("chemicalComplianceStandard")
            .optional_pct("recycledContentPct")
            .optional_non_negative("carbonFootprintKgCo2e")
            .optional_range("repairScore", 0.0, 10.0)
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        Ok(fibre_composition::calculate(input))
    }

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        self.validate_input(&input)?;
        Ok(input)
    }
}

export_plugin!(TextilePlugin);

#[cfg(test)]
mod tests;
