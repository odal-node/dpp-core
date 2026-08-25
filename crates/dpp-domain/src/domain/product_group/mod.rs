//! Typed, product group-specific DPP data for the EU ESPR framework.
//!
//! Each EU product group delegated act (Battery Regulation 2023/1542, upcoming Textile DPP, etc.)
//! defines its own mandatory fields. This module contains typed Rust structs for each
//! supported product group and a discriminated union [`ProductGroupData`] that replaces the old opaque
//! `compliance_data: serde_json::Value` field on `Passport`.
//!
//! ## Module layout
//!
//! - [`product_group`] — the [`ProductGroup`] discriminant.
//! - [`enums`]   — cross-product group typed enumerations (chemistry, classes, routes).
//! - [`carbon_footprint`] and [`repairability_score`] — structured environmental metrics.
//! - [`data`]    — one file per product group + the [`ProductGroupData`] union and `redact_product_group_data`.
//! - [`validation`] — thin adapters onto `dpp-rules` cross-field validators.
//!
//! Adding a product group: add `data/{product group}.rs`, a variant to [`ProductGroupData`], an arm
//! to [`ProductGroup`], and (for shared payloads) an entry in `data/shared.rs`.

pub mod carbon_footprint;
pub mod data;
pub mod enums;
#[allow(clippy::module_inception)]
pub mod product_group;
#[cfg(test)]
mod product_group_tests;
pub mod repairability_score;
pub mod validation;

#[cfg(test)]
mod annex_vii_tests;
#[cfg(test)]
mod conformance_tests;
#[cfg(test)]
mod serde_tests;
#[cfg(test)]
mod tests;

pub use carbon_footprint::CarbonFootprint;
pub use data::unsold_goods::{
    CnCategory, CnCategoryError, DiscardReason, DiscardedProductLine, DiscardedQuantity,
    DisclosingEntity, DisclosureScope, FinancialYear, LegalEntityIdentifier, WasteTreatmentSplit,
};
pub use data::{
    AluminiumData, BatteryData, ConstructionData, CriticalRawMaterial, DetergentData,
    DynamicPerformance, ElectronicsData, EnvironmentalReading, ExpectedLifetime, FibreEntry,
    FurnitureData, HarmfulEvents, HazardSymbol, HazardousSubstance, MaterialComposition,
    MattressData, ProductGroupData, StateOfChargeReading, StateOfHealth, SteelData,
    SurfactantEntry, SvhcSubstance, TemperatureRange, TextileData, ToyData, TyreData,
    UnsoldGoodsReport, UsageHistory, redact_product_group_data,
};
pub use enums::{
    BatteryChemistry, BatteryStatus, BatteryType, CarbonFootprintClass, CarbonFootprintClassError,
    DeviceType, EnergyEfficiencyClass, LifecycleStage, ProductionRoute, SystemBoundary,
};
pub use product_group::ProductGroup;
pub use repairability_score::{RepairCriterion, RepairabilityScore};
pub use validation::{
    battery_recycled_chemistry_conflicts, unsold_goods_annex_vii_heading,
    unsold_goods_cn_depth_is_correct, validate_battery_operating_temp, validate_fibre_composition,
    validate_surfactants, validate_svhc_substances,
};
