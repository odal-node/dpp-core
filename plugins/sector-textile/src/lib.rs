//! Textile DPP compliance plugin.
//!
//! NOTE (legacy dual-sector): this one crate currently serves *two* registry
//! sectors — `textile` (fibre composition) and `unsold-goods` (ESPR Article 25
//! destruction ban) — dispatched on the internally-tagged `sector` field of the
//! input. That is the in-payload dispatch smell flagged in the design review:
//! `meta().sector` can only name one sector, so the host cannot cleanly select a
//! dedicated `unsold-goods` plugin. Splitting `unsold-goods` into its own
//! crate is a candidate for the sector-coverage plan — see
//! docs/architecture/DATA-MODEL.md §3.5 and PLUGIN-HOST.md.

mod fibre_composition;
mod unsold_goods;

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    AbiVersion, DppSectorPlugin, PluginCapabilities, PluginCapability, PluginComplianceStatus,
    PluginError, PluginInput, PluginMeta, PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{str_of, Validator};
use serde_json::Value;

#[derive(Default)]
struct TextilePlugin;

/// The unsold-goods report is distinguished by the internally-tagged sector
/// discriminant carried on `SectorData`.
fn is_unsold(input: &PluginInput) -> bool {
    str_of(input, "sector") == Some("unsoldGoods")
}

impl DppSectorPlugin for TextilePlugin {
    fn meta(&self) -> PluginMeta {
        PluginMeta {
            sector: "textile".into(),
            name: "Odal Node Textile Plugin".into(),
            version: env!("CARGO_PKG_VERSION").into(),
            license: "Apache-2.0".into(),
            description: Some(
                "EU textile DPP fibre composition + ESPR Article 22 unsold goods".into(),
            ),
            author: Some("Odal Node".into()),
            homepage: Some("https://github.com/odal-node/dpp-core".into()),
        }
    }

    fn capabilities(&self) -> PluginCapabilities {
        PluginCapabilities {
            abi_version: AbiVersion::current(),
            // Declares the textile schema range. The unsold-goods path is
            // legacy (see module note) and not represented here.
            supported_schemas: vec![SchemaVersionRange {
                min_version: "1.0.0".into(),
                max_version: "1.1.0".into(),
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
        if is_unsold(input) {
            Validator::new(input)
                .require_str("reportingPeriod")
                .require_non_negative("volumeKg")
                .require_str("productCategory")
                .require_str("reason")
                .require_str("destination")
                .require_country("countryOfDisposal")
                .finish()
        } else {
            Validator::new(input)
                .require_non_empty_array("fibreComposition")
                .require_country("countryOfManufacturing")
                .require_str("careInstructions")
                .require_str("chemicalComplianceStandard")
                .finish()
        }
    }

    fn calculate_metrics(&self, input: &PluginInput) -> Result<PluginResult, PluginError> {
        self.validate_input(input)?;
        Ok(if is_unsold(input) {
            unsold_goods::calculate(input)
        } else {
            fibre_composition::calculate(input)
        })
    }

    fn generate_passport(&self, input: &PluginInput) -> Result<Value, PluginError> {
        self.validate_input(input)?;
        Ok(input.clone())
    }
}

export_plugin!(TextilePlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn textile() -> Value {
        json!({
            "fibreComposition": [
                { "fibre": "cotton", "pct": 60.0 },
                { "fibre": "polyester", "pct": 40.0 }
            ],
            "countryOfManufacturing": "BD",
            "careInstructions": "Machine wash 40C",
            "chemicalComplianceStandard": "OEKO-TEX 100",
            "recycledContentPct": 30.0
        })
    }

    fn unsold() -> Value {
        json!({
            "sector": "unsoldGoods",
            "reportingPeriod": "2026-Q2",
            "volumeKg": 120.0,
            "productCategory": "apparel",
            "reason": "end_of_season",
            "destination": "donation",
            "countryOfDisposal": "MK"
        })
    }

    #[test]
    fn fibre_sum_100_is_compliant() {
        let r = TextilePlugin.calculate_metrics(&textile()).unwrap();
        assert_eq!(r.compliance_status, PluginComplianceStatus::Compliant);
        assert_eq!(r.recycled_content_pct(), Some(30.0));
    }

    #[test]
    fn fibre_sum_off_is_non_compliant() {
        let mut d = textile();
        d["fibreComposition"] = json!([{ "fibre": "cotton", "pct": 50.0 }]);
        assert_eq!(
            TextilePlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn missing_fibre_composition_fails_validation() {
        let mut d = textile();
        d.as_object_mut().unwrap().remove("fibreComposition");
        assert!(TextilePlugin.validate_input(&d).is_err());
    }

    #[test]
    fn unsold_donation_is_compliant() {
        assert_eq!(
            TextilePlugin
                .calculate_metrics(&unsold())
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::Compliant
        );
    }

    #[test]
    fn unsold_exempt_without_justification_is_non_compliant() {
        let mut d = unsold();
        d["destination"] = json!("exempt_destruction");
        assert_eq!(
            TextilePlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }
}
