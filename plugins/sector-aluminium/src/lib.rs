//! Aluminium sector plugin — EU ESPR, CBAM-aligned carbon intensity.
//!
//! Thresholds (kg CO₂e per tonne) are production-route dependent:
//! primary (Hall-Héroult) ≤ 10 000, secondary-recycled ≤ 1 000, mixed ≤ 5 000.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    AbiVersion, DppSectorPlugin, PluginCapabilities, PluginCapability, PluginComplianceStatus,
    PluginError, PluginInput, PluginMeta, PluginResult, SchemaVersionRange,
    METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT,
};
use dpp_plugin_sdk::validate::{num, str_of, Validator};
use serde_json::{json, Value};

#[derive(Default)]
struct AluminiumPlugin;

impl DppSectorPlugin for AluminiumPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            sector: "aluminium".into(),
            name: "Odal Node Aluminium Plugin".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            license: "Apache-2.0".into(),
            description: Some("EU ESPR aluminium carbon-intensity validation".into()),
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
            .require_str("alloyGrade")
            .require_str("productionRoute")
            .require_non_negative("co2ePerTonneKg")
            .require_pct("recycledContentPct")
            .require_country("countryOfProduction")
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
            _ => 12_000.0, // conservative default
        };
        let status = if co2e_kg.is_some_and(|v| v <= threshold_kg) {
            PluginComplianceStatus::Compliant
        } else {
            PluginComplianceStatus::NonCompliant
        };
        Ok(PluginResult::new(status)
            .maybe_metric(METRIC_CO2E_SCORE, co2e_kg)
            .maybe_metric(METRIC_RECYCLED_CONTENT_PCT, recycled)
            .with_extra(json!({
                "productionRoute": route,
                "thresholdKgCo2ePerTonne": threshold_kg,
            })))
    }

    fn generate_passport(&self, input: &PluginInput) -> Result<Value, PluginError> {
        self.validate_input(input)?;
        Ok(input.clone())
    }
}

export_plugin!(AluminiumPlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({
            "gtin": "12345678901231",
            "alloyGrade": "6xxx",
            "productionRoute": "secondary-recycled",
            "co2ePerTonneKg": 600.0,
            "recycledContentPct": 75.0,
            "countryOfProduction": "NO"
        })
    }

    #[test]
    fn recycled_route_under_threshold_is_compliant() {
        assert_eq!(
            AluminiumPlugin
                .calculate_metrics(&valid())
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::Compliant
        );
    }

    #[test]
    fn primary_route_over_threshold_non_compliant() {
        let mut d = valid();
        d["productionRoute"] = json!("primary");
        d["co2ePerTonneKg"] = json!(15_000.0);
        assert_eq!(
            AluminiumPlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn out_of_range_recycled_pct_fails() {
        let mut d = valid();
        d["recycledContentPct"] = json!(120.0);
        assert!(AluminiumPlugin.validate_input(&d).is_err());
    }
}
