//! Mattress product group plugin — EU ESPR Working Plan 2025–2030.
//!
//! Validates mandatory declaration fields and stores manufacturer-supplied
//! environmental data. The working plan selects mattresses separately from
//! furniture, so no delegated act exists for either and compliance thresholds
//! are pending: the determination is `NOT_ASSESSED`.
//!
//! The checks are furniture's with `productType` removed, because the mattress
//! schema is furniture's field set with that one field removed and nothing
//! added. They are a plugin of their own rather than furniture's because
//! furniture's would have to branch on a category that no longer exists, and
//! because the two groups will be governed by different acts on different
//! timetables.

use dpp_plugin_sdk::export_plugin;
use dpp_plugin_sdk::traits::{
    DppProductGroupPlugin, METRIC_CO2E_SCORE, METRIC_RECYCLED_CONTENT_PCT,
    METRIC_REPAIRABILITY_INDEX, PluginComplianceStatus, PluginError, PluginIdentity, PluginInput,
    PluginResult, SchemaVersionRange,
};
use dpp_plugin_sdk::validate::{Validator, num};
use serde_json::Value;

#[cfg(test)]
mod tests;

/// The values the mattress schema allows for `primaryMaterial`.
///
/// A copy of the schema's `enum`, because a plugin that accepts a value its own
/// schema refuses declares itself satisfied with input the passport then cannot
/// carry. `tests::schema::primary_materials_are_the_schemas` holds the two equal.
const PRIMARY_MATERIALS: &[&str] = &[
    "solid-wood",
    "engineered-wood",
    "metal",
    "upholstered",
    "mixed",
    "other",
];

#[derive(Default)]
struct MattressPlugin;

impl DppProductGroupPlugin for MattressPlugin {
    fn plugin_identity(&self) -> PluginIdentity {
        PluginIdentity {
            product_group: "mattress",
            name: "Odal Node Mattress Plugin",
            version: env!("CARGO_PKG_VERSION"),
            description: "EU ESPR mattress validation and metrics",
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
            .require_product_identifier("productIdentifier")
            .require_enum("primaryMaterial", PRIMARY_MATERIALS)
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

export_plugin!(MattressPlugin);
