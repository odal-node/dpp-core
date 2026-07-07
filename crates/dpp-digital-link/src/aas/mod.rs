//! Asset Administration Shell (AAS) mapping for DPP interoperability.
//!
//! The IDTA AAS metamodel (IDTA-01001-3-0) is the interoperability standard
//! chosen by EU Industry 4.0 and Catena-X data spaces. DPPs published through
//! Odal Node must be expressible as AAS shells + submodels for ecosystem
//! registry interoperability.
//!
//! ## Key types
//!
//! - [`AasShell`] — top-level container; holds [`AssetInformation`] (linking the
//!   shell to the physical product via GS1 GTIN) and references to its submodels.
//! - [`AasSubmodel`] — one logical grouping of product data, e.g.,
//!   `ProductIdentification`, `EnvironmentalImpact`, `BatteryTechnicalData`.
//! - [`AasSubmodelElement`] — leaf value ([`AasProperty`]), group
//!   ([`AasCollection`]), or external link ([`AasReference`]).
//!
//! ## Primary entry point
//!
//! [`build_aas_from_passport`] maps a typed [`Passport`](dpp_domain::Passport) + a GS1 GTIN string
//! into a complete `(AasShell, Vec<AasSubmodel>)` ready for serialisation or
//! registry submission. The shell references its submodels by ID following the
//! IDTA AAS Part 2 API pattern — shell and submodels are served from separate
//! endpoints.
//!
//! Always produces five core submodels:
//! `ProductIdentification`, `ManufacturerInformation`, `EnvironmentalImpact`,
//! `MaterialComposition`, `Repairability`. If `passport.sector_data` is `Some`,
//! a sixth sector-specific submodel is appended (`BatteryTechnicalData`,
//! `TextileMaterialDeclaration`, `ElectronicsProductData`, or a generic
//! `SectorData` fallback for sectors without a dedicated template yet).
//!
//! The lower-level [`map_dpp_to_aas_submodel`] remains available as a generic
//! JSON-to-submodel escape hatch.

pub mod semantic_ids;

mod builder;
mod mapper;
mod model;
mod property;
mod sectors;
mod templates;
#[cfg(test)]
mod tests;

pub use builder::build_aas_from_passport;
pub use mapper::map_dpp_to_aas_submodel;
pub use model::{
    AasCollection, AasDataType, AasProperty, AasReference, AasSemId, AasSemIdKey, AasShell,
    AasSubmodel, AasSubmodelElement, AasSubmodelRef, AssetInformation, SpecificAssetId,
};
pub use property::{boolean_property, double_property, integer_property, string_property};
pub use templates::{SubmodelTemplate, placeholder_templates, sector_submodel_template};
