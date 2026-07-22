//! Construction products sector plugin — EU CPR 2024/3110.
//!
//! Validates the mandatory declaration fields and stores manufacturer-supplied
//! carbon footprint values. Compliance thresholds are pending final delegated
//! acts (~2028–2032), so the determination is `NOT_ASSESSED`.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppSectorPlugin, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT, PluginComplianceStatus,
    PluginError, PluginIdentity, PluginInput, PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, num};
use serde_json::Value;

#[derive(Default)]
struct ConstructionPlugin;

impl DppSectorPlugin for ConstructionPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            sector: "construction",
            name: "Odal Node Construction Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU CPR 2024/3110 construction product validation",
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
            .require_str("productFamily")
            .require_country("countryOfOrigin")
            .require_non_negative("co2ePerFunctionalUnitKg")
            .require_str("functionalUnit")
            .optional_pct("recycledContentPct")
            .finish()
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        Ok(PluginResult::new(PluginComplianceStatus::NotAssessed)
            .maybe_metric(METRIC_CO2E_SCORE, num(input, "co2ePerFunctionalUnitKg"))
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

export_plugin!(ConstructionPlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({
            "gtin": "12345678901231",
            "productFamily": "cement",
            "countryOfOrigin": "PL",
            "co2ePerFunctionalUnitKg": 780.0,
            "functionalUnit": "per tonne"
        })
    }

    #[test]
    fn valid_input_is_not_assessed_with_co2e() {
        let r = ConstructionPlugin.calculate_metrics(&valid()).unwrap();
        assert_eq!(r.compliance_status, PluginComplianceStatus::NotAssessed);
        assert_eq!(r.co2e_score(), Some(780.0));
    }

    #[test]
    fn missing_functional_unit_fails() {
        let mut d = valid();
        d.as_object_mut().unwrap().remove("functionalUnit");
        assert!(ConstructionPlugin.validate_input(&d).is_err());
    }
}
