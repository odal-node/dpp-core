//! [`build_aas_from_passport`] — the primary entry point mapping a passport to
//! a complete AAS shell + submodels.

use dpp_domain::Passport;

use super::model::{AasShell, AasSubmodel, AasSubmodelRef, AssetInformation, SpecificAssetId};
use super::sectors;

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
