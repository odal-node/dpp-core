//! Typed, sector-specific DPP data for the EU ESPR framework.
//!
//! Each EU sector delegated act (Battery Regulation 2023/1542, upcoming Textile DPP, etc.)
//! defines its own mandatory fields. This module contains typed Rust structs for each
//! supported sector and a discriminated union [`SectorData`] that replaces the old opaque
//! `compliance_data: serde_json::Value` field on `Passport`.
//!
//! ## Module layout
//!
//! - this `mod.rs` — the [`Sector`] discriminant.
//! - [`enums`]   — cross-sector typed enumerations (chemistry, classes, routes).
//! - [`metrics`] — structured environmental metrics ([`CarbonFootprint`], [`RepairabilityScore`]).
//! - [`data`]    — one file per sector + the [`SectorData`] union and `redact_sector_data`.
//! - [`validation`] — thin adapters onto `dpp-rules` cross-field validators.
//!
//! Adding a sector: add `data/{sector}.rs`, a variant to [`SectorData`], an arm
//! to [`Sector`], and (for shared payloads) an entry in `data/shared.rs`.

use serde::{Deserialize, Serialize};

pub mod data;
pub mod enums;
pub mod metrics;
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
pub use validation::{validate_fibre_composition, validate_surfactants, validate_svhc_substances};

/// EU ESPR product sector — determines which delegated act schema applies.
///
/// Used by the compliance infrastructure to dispatch to the correct
/// `ComplianceStrategy` implementation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub enum Sector {
    Battery,
    Textile,
    TextileUnsoldGoods,
    Steel,
    Electronics,
    Construction,
    Tyre,
    Toy,
    Aluminium,
    Furniture,
    Detergent,
    Other,
}

impl Sector {
    /// Minimum data retention period in years as required by the applicable
    /// EU delegated act.  The Battery Regulation (2023/1542) mandates ≥ 10
    /// years after end-of-life.  Other sectors default to 10 years pending
    /// their respective delegated acts.
    pub const fn minimum_retention_years(&self) -> u32 {
        match self {
            Self::Battery => 10,
            Self::Textile | Self::TextileUnsoldGoods => 10,
            Self::Steel => 10,
            Self::Electronics => 10,
            Self::Construction => 10,
            Self::Tyre => 10,
            Self::Toy => 10,
            Self::Aluminium => 10,
            Self::Furniture => 10,
            Self::Detergent => 10,
            Self::Other => 10,
        }
    }

    /// Canonical sector key used by the schema registry and the `SectorCatalog`.
    ///
    /// This is the one true spelling (kebab-case where needed), distinct from
    /// the enum's camelCase serde tag — e.g. `TextileUnsoldGoods` serialises as
    /// `"textileUnsoldGoods"` but its catalog/registry key is `"textile-unsold"`.
    pub const fn catalog_key(&self) -> &'static str {
        match self {
            Self::Battery => "battery",
            Self::Textile => "textile",
            Self::TextileUnsoldGoods => "textile-unsold",
            Self::Steel => "steel",
            Self::Electronics => "electronics",
            Self::Construction => "construction",
            Self::Tyre => "tyre",
            Self::Toy => "toy",
            Self::Aluminium => "aluminium",
            Self::Furniture => "furniture",
            Self::Detergent => "detergent",
            Self::Other => "other",
        }
    }
}
