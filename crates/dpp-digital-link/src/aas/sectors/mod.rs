//! Sector-specific AAS submodel builders, dispatched by [`dispatch::build_sector_submodel`].
//!
//! One file per sector (`battery`, `textile`, `electronics`, …) plus the five
//! cross-sector builders below (`build_product_identification_submodel` and
//! siblings), which every sector shares regardless of `passport.sector` and
//! so are not sector files themselves.
//!
//! **Deviation, accepted:** the five shared builders keep their logic here
//! rather than in their own files (rule 2 target would be a `common.rs`).
//! `dispatch.rs` was extracted per the pack's F2 finding; the shared builders
//! were left as they are not the dispatch logic that finding named. Revisit
//! if this file grows past the sector-file count.

mod aluminium;
mod battery;
mod construction;
mod detergent;
mod dispatch;
mod electronics;
mod furniture;
mod steel;
mod textile;
mod toy;
mod tyre;
mod unsold_goods;

use dpp_domain::Passport;

use crate::aas::model::{AasCollection, AasReference, AasSemId, AasSubmodel, AasSubmodelElement};
use crate::aas::property::{double_property, string_property};
use crate::aas::semantic_ids;

pub(super) use dispatch::build_sector_submodel;

pub(super) fn build_product_identification_submodel(passport: &Passport) -> AasSubmodel {
    let mut elements = vec![
        string_property(
            "productName",
            &passport.product_name,
            Some(semantic_ids::PRODUCT_IDENTIFICATION),
            None,
        ),
        string_property("sector", passport.sector.catalog_key(), None, None),
        string_property("passportId", &passport.id.to_string(), None, None),
        string_property("schemaVersion", &passport.schema_version, None, None),
    ];
    if let Some(batch) = &passport.batch_id {
        elements.push(string_property("batchId", batch, None, None));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{}:product-identification", passport.id),
        id_short: "ProductIdentification".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::PRODUCT_IDENTIFICATION)),
        submodel_elements: elements,
    }
}

pub(super) fn build_manufacturer_submodel(passport: &Passport) -> AasSubmodel {
    let mfr = &passport.manufacturer;
    let mut elements = vec![
        string_property(
            "name",
            &mfr.name,
            Some(semantic_ids::MANUFACTURER_INFORMATION),
            None,
        ),
        string_property("address", &mfr.address, None, None),
    ];
    if let Some(url) = &mfr.did_web_url {
        elements.push(AasSubmodelElement::Reference(AasReference {
            id_short: "didWebUrl".into(),
            value: url.clone(),
            semantic_id: None,
        }));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{}:manufacturer-information", passport.id),
        id_short: "ManufacturerInformation".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::MANUFACTURER_INFORMATION)),
        submodel_elements: elements,
    }
}

pub(super) fn build_environmental_impact_submodel(passport: &Passport) -> AasSubmodel {
    let mut elements = Vec::new();
    if let Some(ref cf) = passport.co2e_per_unit {
        elements.push(double_property(
            "co2ePerUnit",
            cf.value_kg,
            Some(semantic_ids::CARBON_FOOTPRINT),
            Some("kgCO2e"),
        ));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{}:environmental-impact", passport.id),
        id_short: "EnvironmentalImpact".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::CARBON_FOOTPRINT)),
        submodel_elements: elements,
    }
}

pub(super) fn build_material_composition_submodel(passport: &Passport) -> AasSubmodel {
    let elements = passport
        .materials
        .iter()
        .enumerate()
        .map(|(i, mat)| {
            let mut mat_elems = vec![
                string_property("name", &mat.name, None, None),
                double_property("weightKg", mat.weight_kg, None, Some("kg")),
            ];
            if let Some(pct) = mat.recycled_pct {
                mat_elems.push(double_property("recycledPct", pct, None, Some("%")));
            }
            if let Some(ref country) = mat.origin_country {
                mat_elems.push(string_property("originCountry", country, None, None));
            }
            AasSubmodelElement::SubmodelElementCollection(AasCollection {
                id_short: format!("material_{i}"),
                value: mat_elems,
                semantic_id: None,
            })
        })
        .collect();
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{}:material-composition", passport.id),
        id_short: "MaterialComposition".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::MATERIAL_COMPOSITION)),
        submodel_elements: elements,
    }
}

pub(super) fn build_repairability_submodel(passport: &Passport) -> AasSubmodel {
    let mut elements = Vec::new();
    if let Some(ref rs) = passport.repairability_score {
        elements.push(double_property(
            "repairabilityScore",
            rs.overall,
            Some(semantic_ids::REPAIRABILITY),
            Some("index 0-10"),
        ));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{}:repairability", passport.id),
        id_short: "Repairability".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::REPAIRABILITY)),
        submodel_elements: elements,
    }
}
