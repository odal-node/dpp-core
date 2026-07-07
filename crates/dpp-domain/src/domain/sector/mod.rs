//! Typed, sector-specific DPP data for the EU ESPR framework.
//!
//! Each EU sector delegated act (Battery Regulation 2023/1542, upcoming Textile DPP, etc.)
//! defines its own mandatory fields. This module contains typed Rust structs for each
//! supported sector and a discriminated union [`SectorData`] that replaces the old opaque
//! `compliance_data: serde_json::Value` field on `Passport`.
//!
//! ## Module layout
//!
//! - [`sector`] — the [`Sector`] discriminant.
//! - [`enums`]   — cross-sector typed enumerations (chemistry, classes, routes).
//! - [`metrics`] — structured environmental metrics ([`CarbonFootprint`], [`RepairabilityScore`]).
//! - [`data`]    — one file per sector + the [`SectorData`] union and `redact_sector_data`.
//! - [`validation`] — thin adapters onto `dpp-rules` cross-field validators.
//!
//! Adding a sector: add `data/{sector}.rs`, a variant to [`SectorData`], an arm
//! to [`Sector`], and (for shared payloads) an entry in `data/shared.rs`.

pub mod data;
pub mod enums;
pub mod metrics;
#[allow(clippy::module_inception)]
pub mod sector;
pub mod validation;

#[cfg(test)]
mod tests;

pub use data::{
    AluminiumData, BatteryData, ConstructionData, CriticalRawMaterial, DetergentData,
    ElectronicsData, FibreEntry, FurnitureData, MaterialComposition, SectorData, SteelData,
    SurfactantEntry, SvhcSubstance, TextileData, ToyData, TyreData, UnsoldGoodsDestination,
    UnsoldGoodsReason, UnsoldGoodsReport, redact_sector_data,
};
pub use enums::{
    BatteryChemistry, BatteryType, CarbonFootprintClass, EnergyEfficiencyClass, LifecycleStage,
    ProductionRoute, SystemBoundary,
};
pub use metrics::{CarbonFootprint, RepairCriterion, RepairabilityScore};
pub use sector::Sector;
pub use validation::{
    battery_recycled_chemistry_conflicts, validate_battery_operating_temp,
    validate_fibre_composition, validate_surfactants, validate_svhc_substances,
};
