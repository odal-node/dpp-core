//! Iron & Steel sector plugin — EU ESPR carbon intensity and recycled content.
//!
//! CO₂e thresholds (tonne CO₂e per tonne of steel) are production-route
//! dependent: blast-furnace ≤ 2.1 (EU CBAM benchmark), electric-arc ≤ 0.4,
//! direct-reduction ≤ 1.0. "steel" is the **sector**; the steel form
//! (`flat`/`long`/`tube`, carried as `productCategory`) is a product category
//! the plugin records but does not dispatch on.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    AbiVersion, DppSectorPlugin, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT,
    PluginCapabilities, PluginCapability, PluginComplianceStatus, PluginError, PluginInput,
    PluginMeta, PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, num, str_of};
use serde_json::{Value, json};

#[derive(Default)]
struct SteelPlugin;

impl DppSectorPlugin for SteelPlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            sector: "steel".into(),
            name: "Odal Node Steel Plugin".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            license: "Apache-2.0".into(),
            description: Some("EU ESPR steel carbon-intensity validation and metrics".into()),
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
            .require_non_negative("co2ePerTonneSteel")
            .require_pct("recycledScrapContentPct")
            .require_str("productCategory")
            .require_enum(
                "productionRoute",
                &["blast-furnace", "electric-arc", "direct-reduction"],
            )
            .require_country("countryOfProduction")
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        let co2e = num(input, "co2ePerTonneSteel");
        let recycled = num(input, "recycledScrapContentPct");
        let route = str_of(input, "productionRoute").unwrap_or("");
        let threshold = match route {
            "blast-furnace" => 2.1,
            "electric-arc" => 0.4,
            "direct-reduction" => 1.0,
            // Unreachable after validate_input rejects unknown routes; fail
            // closed on the strictest threshold rather than the most permissive.
            _ => 0.4,
        };
        let status = if co2e.is_some_and(|v| v <= threshold) {
            PluginComplianceStatus::Compliant
        } else {
            PluginComplianceStatus::NonCompliant
        };
        Ok(PluginResult::new(status)
            .maybe_metric(METRIC_CO2E_SCORE, co2e)
            .maybe_metric(METRIC_RECYCLED_CONTENT_PCT, recycled)
            .with_extra(json!({
                "productionRoute": route,
                "thresholdTco2ePerTonne": threshold,
            })))
    }

    fn generate_passport(&self, input: &PluginInput) -> Result<Value, PluginError> {
        self.validate_input(input)?;
        Ok(input.clone())
    }
}

export_plugin!(SteelPlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({
            "gtin": "12345678901231",
            "co2ePerTonneSteel": 0.35,
            "recycledScrapContentPct": 90.0,
            "productCategory": "long",
            "productionRoute": "electric-arc",
            "countryOfProduction": "DE"
        })
    }

    #[test]
    fn electric_arc_low_co2e_is_compliant() {
        let r = SteelPlugin.calculate_metrics(&valid()).unwrap();
        assert_eq!(r.compliance_status, PluginComplianceStatus::Compliant);
        assert_eq!(r.co2e_score(), Some(0.35));
    }

    #[test]
    fn blast_furnace_over_threshold_non_compliant() {
        let mut d = valid();
        d["productionRoute"] = json!("blast-furnace");
        d["co2ePerTonneSteel"] = json!(2.9);
        assert_eq!(
            SteelPlugin.calculate_metrics(&d).unwrap().compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn missing_country_fails_validation() {
        let mut d = valid();
        d.as_object_mut().unwrap().remove("countryOfProduction");
        assert!(SteelPlugin.validate_input(&d).is_err());
    }

    #[test]
    fn unrecognized_production_route_fails_validation() {
        // A wrong-case or unlisted route must be rejected, not scored against
        // the most-permissive wildcard threshold.
        for route in ["Electric-Arc", "electric_arc", "unknown"] {
            let mut d = valid();
            d["productionRoute"] = json!(route);
            assert!(
                SteelPlugin.validate_input(&d).is_err(),
                "route {route:?} should fail validation"
            );
        }
    }
}
