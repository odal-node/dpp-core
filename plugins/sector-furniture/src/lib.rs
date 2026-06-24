//! Furniture sector plugin — EU ESPR Working Plan 2025–2030.
//!
//! Validates mandatory declaration fields and stores manufacturer-supplied
//! environmental data. Compliance thresholds are pending final delegated acts
//! (~2028–2031), so the determination is `NOT_ASSESSED`.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    AbiVersion, DppSectorPlugin, PluginCapabilities, PluginCapability, PluginComplianceStatus,
    PluginError, PluginInput, PluginMeta, PluginResult, SchemaVersionRange,
    METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, METRIC_REPAIRABILITY_INDEX,
};
use dpp_plugin_sdk::validate::{num, Validator};
use serde_json::Value;

#[derive(Default)]
struct FurniturePlugin;

impl DppSectorPlugin for FurniturePlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            sector: "furniture".into(),
            name: "Odal Node Furniture Plugin".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            license: "Apache-2.0".into(),
            description: Some("EU ESPR furniture validation and metrics".into()),
            author: Some("Odal Node".into()),
            homepage: Some("https://github.com/odal-node/dpp-core".into()),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            abi_version: AbiVersion::current(),
            supported_schemas: vec![SchemaVersionRange {
                min_version: "1.0.0".into(),
                max_version: "1.0.0".into(),
            }],
            capabilities: vec![
                PluginCapability::Validate,
                PluginCapability::ComputeMetrics,
                PluginCapability::GeneratePassport,
            ],
            min_host_version: None,
            max_fuel: None,
            max_memory_bytes: None,
        }
    }

    fn validate_input(&self, input: &PluginInput) -> Result<(), PluginError> {
        Validator::new(input)
            .require_gtin("gtin")
            .require_str("productType")
            .require_str("primaryMaterial")
            .require_country("countryOfManufacture")
            .optional_pct("recycledContentPct")
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        Ok(PluginResult::new(PluginComplianceStatus::NotAssessed)
            .maybe_metric(METRIC_CO2E_SCORE, num(input, "co2ePerUnitKg"))
            .maybe_metric(METRIC_REPAIRABILITY_INDEX, num(input, "repairabilityScore"))
            .maybe_metric(METRIC_RECYCLED_CONTENT_PCT, num(input, "recycledContentPct")))
    }

    fn generate_passport(&self, input: &PluginInput) -> Result<Value, PluginError> {
        self.validate_input(input)?;
        Ok(input.clone())
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
            "countryOfManufacture": "SE",
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
        d["countryOfManufacture"] = json!("Sweden");
        assert!(FurniturePlugin.validate_input(&d).is_err());
    }
}
