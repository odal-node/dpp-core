//! Detergent & surfactant sector plugin — EU Regulation 2026/405.
//!
//! Hard check available today: all surfactants must be readily biodegradable.
//! Any surfactant declaring `biodegradable: false` makes the product
//! `NON_COMPLIANT`. Full DPP thresholds (mandate 2029) are pending.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    AbiVersion, DppSectorPlugin, PluginCapabilities, PluginCapability, PluginComplianceStatus,
    PluginError, PluginInput, PluginMeta, PluginResult, SchemaVersionRange,
    METRIC_CO2E_SCORE,
};
use dpp_plugin_sdk::validate::{num, Validator};
use serde_json::Value;

#[derive(Default)]
struct DetergentPlugin;

impl DppSectorPlugin for DetergentPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            sector: "detergent".into(),
            name: "Odal Node Detergent Plugin".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            license: "Apache-2.0".into(),
            description: Some("EU 2026/405 detergent biodegradability validation".into()),
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
            .require_str("format")
            .require_non_empty_array("surfactants")
            .require_country("countryOfManufacture")
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        let has_non_biodegradable = input
            .get("surfactants")
            .and_then(Value::as_array)
            .is_some_and(|arr| {
                arr.iter()
                    .any(|s| s.get("biodegradable").and_then(Value::as_bool) == Some(false))
            });
        let status = if has_non_biodegradable {
            PluginComplianceStatus::NonCompliant
        } else {
            PluginComplianceStatus::NotAssessed
        };
        Ok(PluginResult::new(status)
            .maybe_metric(METRIC_CO2E_SCORE, num(input, "co2ePerUnitKg")))
    }

    fn generate_passport(&self, input: &PluginInput) -> Result<Value, PluginError> {
        self.validate_input(input)?;
        Ok(input.clone())
    }
}

export_plugin!(DetergentPlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({
            "gtin": "12345678901231",
            "productType": "laundry",
            "format": "liquid",
            "surfactants": [
                { "name": "LAS", "biodegradable": true, "concentrationBand": "5-15%" }
            ],
            "countryOfManufacture": "DE",
            "co2ePerUnitKg": 1.2
        })
    }

    #[test]
    fn all_biodegradable_is_not_assessed() {
        assert_eq!(
            DetergentPlugin
                .calculate_metrics(&valid())
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NotAssessed
        );
    }

    #[test]
    fn non_biodegradable_surfactant_is_non_compliant() {
        let mut d = valid();
        d["surfactants"][0]["biodegradable"] = json!(false);
        assert_eq!(
            DetergentPlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn empty_surfactants_fails_validation() {
        let mut d = valid();
        d["surfactants"] = json!([]);
        assert!(DetergentPlugin.validate_input(&d).is_err());
    }
}
