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
//! [`build_aas_from_passport`] maps a typed [`Passport`] + a GS1 GTIN string
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

mod mapper;
mod model;
mod property;
mod sectors;
mod templates;
#[cfg(test)]
mod tests;

pub use mapper::map_dpp_to_aas_submodel;
pub use model::{
    AasCollection, AasDataType, AasProperty, AasReference, AasSemId, AasSemIdKey, AasShell,
    AasSubmodel, AasSubmodelElement, AasSubmodelRef, AssetInformation, SpecificAssetId,
};
pub use property::{boolean_property, double_property, integer_property, string_property};
pub use templates::{SubmodelTemplate, placeholder_templates, sector_submodel_template};

use dpp_domain::Passport;

/// Map a typed [`Passport`] and its GS1 GTIN into a complete AAS shell + submodels.
///
/// Returns `(AasShell, Vec<AasSubmodel>)`. The shell's `submodels` list
/// contains only ID references; the actual submodel payloads are in the `Vec`.
///
/// `gtin` is the 14-digit GTIN identifying the product model. It becomes the
/// `globalAssetId` and a `specificAssetId` entry for GS1 Digital Link routing.
pub fn build_aas_from_passport(passport: &Passport, gtin: &str) -> (AasShell, Vec<AasSubmodel>) {
    let passport_id = passport.id.to_string();

    let mut specific_asset_ids = vec![
        SpecificAssetId {
            name: "gtin".into(),
            value: gtin.to_owned(),
        },
        SpecificAssetId {
            name: "serialId".into(),
            value: passport_id.clone(),
        },
    ];
    if let Some(batch) = &passport.batch_id {
        specific_asset_ids.push(SpecificAssetId {
            name: "batchId".into(),
            value: batch.clone(),
        });
    }

    let mut submodels = vec![
        sectors::build_product_identification_submodel(passport),
        sectors::build_manufacturer_submodel(passport),
        sectors::build_environmental_impact_submodel(passport),
        sectors::build_material_composition_submodel(passport),
        sectors::build_repairability_submodel(passport),
    ];
    if let Some(sd) = &passport.sector_data {
        submodels.push(sectors::build_sector_submodel(sd, &passport_id));
    }

    let shell = AasShell {
        id: format!("urn:odal-node:aas:{passport_id}"),
        id_short: "DigitalProductPassport".into(),
        model_type: "AssetAdministrationShell".into(),
        kind: "Instance".into(),
        asset_information: AssetInformation {
            global_asset_id: format!("urn:odal-node:product:{gtin}"),
            specific_asset_ids,
        },
        submodels: submodels
            .iter()
            .map(|s| AasSubmodelRef { id: s.id.clone() })
            .collect(),
    };

    (shell, submodels)
}
