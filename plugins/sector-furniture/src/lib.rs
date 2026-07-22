//! Furniture sector plugin — EU ESPR Working Plan 2025–2030.
//!
//! Validates mandatory declaration fields and stores manufacturer-supplied
//! environmental data. Compliance thresholds are pending final delegated acts
//! (~2028–2031), so the determination is `NOT_ASSESSED`.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppSectorPlugin, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, METRIC_REPAIRABILITY_INDEX,
    PluginComplianceStatus, PluginError, PluginIdentity, PluginInput, PluginResult,
    SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, num};
use serde_json::Value;

#[derive(Default)]
struct FurniturePlugin;

impl DppSectorPlugin for FurniturePlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            sector: "furniture",
            name: "Odal Node Furniture Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU ESPR furniture validation and metrics",
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
            .require_gtin("gtin")
            .require_str("productType")
            .require_str("primaryMaterial")
            .require_country("countryOfOrigin")
            .optional_pct("recycledContentPct")
            .optional_non_negative("co2ePerUnitKg")
            .optional_range("repairabilityScore", 0.0, 10.0)
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        Ok(PluginResult::new(PluginComplianceStatus::NotAssessed)
            .maybe_metric(METRIC_CO2E_SCORE, num(input, "co2ePerUnitKg"))
            .maybe_metric(METRIC_REPAIRABILITY_INDEX, num(input, "repairabilityScore"))
            .maybe_metric(
                METRIC_RECYCLED_CONTENT_PCT,
                num(input, "recycledContentPct"),
            ))
    }

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        self.validate_input(&input)?;
        Ok(input)
    }
}

export_plugin!(FurniturePlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({
            "gtin": "12345678901231",
            "productType": "chair",
            "primaryMaterial": "solid-wood",
            "countryOfOrigin": "SE",
            "co2ePerUnitKg": 22.0,
            "repairabilityScore": 8.0
        })
    }

    #[test]
    fn valid_input_surfaces_metrics() {
        let r = FurniturePlugin.calculate_metrics(&valid()).unwrap();
        assert_eq!(r.co2e_score(), Some(22.0));
        assert_eq!(r.repairability_index(), Some(8.0));
        assert_eq!(r.compliance_status, PluginComplianceStatus::NotAssessed);
    }

    #[test]
    fn invalid_country_fails() {
        let mut d = valid();
        d["countryOfOrigin"] = json!("Sweden");
        assert!(FurniturePlugin.validate_input(&d).is_err());
    }

    #[test]
    fn negative_metrics_are_rejected() {
        let mut d = valid();
        d["co2ePerUnitKg"] = json!(-999.0);
        assert!(FurniturePlugin.validate_input(&d).is_err());

        let mut d = valid();
        d["repairabilityScore"] = json!(-1.0);
        assert!(FurniturePlugin.validate_input(&d).is_err());
    }
}
