//! Textile DPP compliance plugin.
//!
//! NOTE (legacy dual-product group): this one crate currently serves *two* registry
//! product groups — `textile` (fibre composition) and `unsold-goods` (ESPR Article 25
//! destruction ban) — dispatched on the internally-tagged `product_group` field of the
//! input. That is the in-payload dispatch smell flagged in the design review:
//! `meta().product_group` can only name one product group, so the host cannot cleanly select a
//! dedicated `unsold-goods` plugin. Splitting `unsold-goods` into its own
//! crate is a candidate for the product group-coverage plan — see
//! docs/architecture/DATA-MODEL.md §3.4 and PLUGIN-HOST.md.

mod fibre_composition;
mod unsold_goods;

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppProductGroupPlugin, PluginError, PluginIdentity, PluginInput, PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, str_of};
use serde_json::Value;

#[derive(Default)]
struct TextilePlugin;

/// The unsold-goods report is distinguished by the internally-tagged product group
/// discriminant carried on `ProductGroupData`.
fn is_unsold(input: &PluginInput) -> bool {
    str_of(input, "productGroup") == Some("unsoldGoods")
}

impl DppProductGroupPlugin for TextilePlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            product_group: "textile",
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
            // The Annex I header rows. The repeating body is checked in
            // `unsold_goods::calculate`, which can see across lines — the two
            // rules that matter (the treatment split, and point (h)'s
            // subordination) are not answerable field by field.
            Validator::new(input)
                .require_non_empty_array("lines")
                .require_str("measuresTaken")
                .require_str("measuresPlanned")
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

    /// A disclosure in the Annex I shape of Impl. Reg. (EU) 2026/2.
    fn unsold() -> Value {
        json!({
            "productGroup": "unsoldGoods",
            "entity": {
                "name": "Example Retail Group SA",
                "identifier": { "type": "euid", "value": "LUB123456789" },
                "scope": { "type": "standalone" }
            },
            "financialYear": { "start": "2027-01-01", "end": "2027-12-31" },
            "lines": [{
                "cnCategories": ["6203"],
                "description": "Men's suits and trousers",
                "unitsDiscarded": { "value": 1200, "estimated": false },
                "weightKg": { "value": 430, "estimated": true },
                "packagingIncluded": false,
                "reason": "damagedOrContaminated",
                "treatment": {
                    "preparingForReusePct": 20,
                    "recyclingPct": 50,
                    "otherRecoveryPct": 20,
                    "disposalPct": 5,
                    "unknownPct": 5
                }
            }],
            "measuresTaken": "Introduced pre-season demand forecasting.",
            "measuresPlanned": "Extending the donation window to twelve weeks."
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
    fn a_consistent_disclosure_is_compliant() {
        assert_eq!(
            TextilePlugin
                .calculate_metrics(&unsold())
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::Compliant
        );
    }

    /// Annex I note (i) provides `unknown` for the share that could not be
    /// established, so nothing is left over and the split must reach 100.
    #[test]
    fn a_treatment_split_that_misses_100_is_non_compliant() {
        let mut d = unsold();
        d["lines"][0]["treatment"]["disposalPct"] = json!(1);
        assert_eq!(
            TextilePlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    /// Del. Reg. (EU) 2026/296 Art. 2 point (h) applies "only where none of the
    /// circumstances referred to in points (a) to (g) are applicable".
    #[test]
    fn donation_claimed_beside_a_stronger_reason_is_non_compliant() {
        let mut d = unsold();
        let mut second = d["lines"][0].clone();
        second["reason"] = json!("offeredForDonationNotAccepted");
        d["lines"].as_array_mut().unwrap().push(second);
        assert_eq!(
            TextilePlugin
                .calculate_metrics(&d)
                .unwrap()
                .compliance_status,
            PluginComplianceStatus::NonCompliant
        );
    }

    /// A disclosure with no lines is structurally invalid, not merely
    /// non-compliant — it is rejected at validation before any determination is
    /// reached.
    #[test]
    fn a_disclosure_with_no_lines_fails_validation() {
        let mut d = unsold();
        d["lines"] = json!([]);
        assert!(TextilePlugin.validate_input(&d).is_err());
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

    /// Annex I notes (i) and (j) ask for the measures themselves; a run of
    /// whitespace is not one, and the validator treats it as absent.
    #[test]
    fn whitespace_only_prevention_measures_fail_validation() {
        let mut d = unsold();
        d["measuresPlanned"] = json!("          ");
        assert!(TextilePlugin.validate_input(&d).is_err());
    }
}
