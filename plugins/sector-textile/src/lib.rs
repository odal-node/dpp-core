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
    DppSectorPlugin, PluginError, PluginIdentity, PluginInput, PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, str_of};
use serde_json::Value;

#[derive(Default)]
struct TextilePlugin;

/// The unsold-goods report is distinguished by the internally-tagged sector
/// discriminant carried on `SectorData`.
fn is_unsold(input: &PluginInput) -> bool {
    str_of(input, "sector") == Some("unsoldGoods")
}

impl DppSectorPlugin for TextilePlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            sector: "textile",
            name: "Odal Node Textile Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU textile DPP fibre composition + ESPR Art. 25 unsold goods",
        }
    }

    // Declares the textile schema range. The unsold-goods path is legacy
    // (see module note) and not represented here.
    fn schema_version_range(&self) -> SchemaVersionRange {
        SchemaVersionRange {
            min_version: "1.0.0".into(),
            max_version: "1.2.0".into(),
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
                .require_country("countryOfOrigin")
                .require_str("careInstructions")
                .require_str("chemicalComplianceStandard")
                .optional_pct("recycledContentPct")
                .optional_non_negative("carbonFootprintKgCo2e")
                .optional_range("repairScore", 0.0, 10.0)
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

    fn generate_passport(&self, input: PluginInput) -> Result<Value, PluginError> {
        self.validate_input(&input)?;
        Ok(input)
    }
}

export_plugin!(TextilePlugin);

#[cfg(test)]
mod tests {
    use super::*;
    use dpp_plugin_sdk::traits::PluginComplianceStatus;
    use serde_json::json;

    fn textile() -> Value {
        json!({
            "fibreComposition": [
                { "fibre": "cotton", "pct": 60.0 },
                { "fibre": "polyester", "pct": 40.0 }
            ],
            "countryOfOrigin": "BD",
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

    #[test]
    fn out_of_range_fibre_pcts_are_non_compliant() {
        // Sums to 100 but neither percentage is physically valid.
        let mut d = textile();
        d["fibreComposition"] = json!([
            { "fibre": "cotton", "pct": 150.0 },
            { "fibre": "wool", "pct": -50.0 }
        ]);
        assert_eq!(
            TextilePlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn fibre_entry_missing_pct_is_non_compliant() {
        // One entry has no pct — an incomplete declaration, not a 100% cotton.
        let mut d = textile();
        d["fibreComposition"] = json!([
            { "fibre": "cotton" },
            { "fibre": "wool", "pct": 100.0 }
        ]);
        assert_eq!(
            TextilePlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    #[test]
    fn negative_carbon_footprint_fails_validation() {
        let mut d = textile();
        d["carbonFootprintKgCo2e"] = json!(-10.0);
        assert!(TextilePlugin.validate_input(&d).is_err());
    }

    #[test]
    fn whitespace_only_justification_is_non_compliant() {
        let mut d = unsold();
        d["destination"] = json!("exempt_destruction");
        d["destructionJustification"] = json!("          "); // 10 spaces
        assert_eq!(
            TextilePlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }
}
