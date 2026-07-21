//! Toy sector plugin — EU Delegated Regulation 2025/2509 + Toy Safety Directive.
//!
//! The one hard check available today is CE marking: a toy declaring
//! `ceMarking: false` is `NON_COMPLIANT`. Full safety/DPP thresholds (mandate
//! 2030) are pending, so otherwise the determination is `NOT_ASSESSED`.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppSectorPlugin, PluginComplianceStatus, PluginError, PluginIdentity, PluginInput,
    PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::Validator;
use serde_json::Value;

#[derive(Default)]
struct ToyPlugin;

impl DppSectorPlugin for ToyPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            sector: "toy",
            name: "Odal Node Toy Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU 2025/2509 toy safety and CE-marking validation",
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
mod tests {
    use super::*;
    use serde_json::json;

    fn valid() -> Value {
        json!({
            "gtin": "12345678901231",
            "ageGroup": "3-6",
            "primaryMaterial": "wood",
            "ceMarking": true,
            "countryOfOrigin": "DE"
        })
    }

    #[test]
    fn ce_marked_toy_is_not_assessed() {
        assert_eq!(
            ToyPlugin
                .calculate_metrics(&valid())
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NotAssessed
        );
    }

    #[test]
    fn missing_ce_marking_is_non_compliant() {
        let mut d = valid();
        d["ceMarking"] = json!(false);
        assert_eq!(
            ToyPlugin.calculate_metrics(&d).unwrap().compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn ce_marking_must_be_bool() {
        let mut d = valid();
        d["ceMarking"] = json!("yes");
        assert!(ToyPlugin.validate_input(&d).is_err());
    }
}
