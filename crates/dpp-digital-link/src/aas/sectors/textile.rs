use dpp_domain::TextileData;

use crate::aas::model::{AasCollection, AasReference, AasSemId, AasSubmodel, AasSubmodelElement};
use crate::aas::property::{double_property, string_property, svhc_substance_element};
use crate::aas::semantic_ids;

pub(super) fn build_textile_submodel(t: &TextileData, passport_id: &str) -> AasSubmodel {
    let mut elements = vec![
        string_property("countryOfOrigin", &t.country_of_origin, None, None),
        string_property("careInstructions", &t.care_instructions, None, None),
        string_property(
            "chemicalComplianceStandard",
            &t.chemical_compliance_standard,
            None,
            None,
        ),
    ];

    let fibre_items = t
        .fibre_composition
        .iter()
        .enumerate()
        .map(|(i, fe)| {
            let mut fe_elems = vec![
                string_property("fibre", &fe.fibre, None, None),
                double_property("pct", fe.pct, None, Some("%")),
            ];
            if let Some(ref country) = fe.country_of_origin {
                fe_elems.push(string_property("countryOfOrigin", country, None, None));
            }
            AasSubmodelElement::SubmodelElementCollection(AasCollection {
                id_short: format!("fibre_{i}"),
                value: fe_elems,
                semantic_id: None,
            })
        })
        .collect();
    elements.push(AasSubmodelElement::SubmodelElementCollection(
        AasCollection {
            id_short: "fibreComposition".into(),
            value: fibre_items,
            semantic_id: None,
        },
    ));

    macro_rules! push_opt_double {
        ($opt:expr, $id:literal, $unit:expr) => {
            if let Some(v) = $opt {
                elements.push(double_property($id, v, None, $unit));
            }
        };
    }

    push_opt_double!(t.recycled_content_pct, "recycledContentPct", Some("%"));
    push_opt_double!(
        t.carbon_footprint_kg_co2e,
        "carbonFootprintKgCo2e",
        Some("kgCO2e")
    );
    push_opt_double!(t.water_use_litres, "waterUseLitres", Some("L"));
    push_opt_double!(
        t.microplastic_shedding_mg_per_wash,
        "microplasticSheddingMgPerWash",
        Some("mg/wash")
    );
    push_opt_double!(t.repair_score, "repairScore", Some("index 0-10"));
    push_opt_double!(t.durability_score, "durabilityScore", Some("index 0-10"));
    push_opt_double!(t.pef_score, "pefScore", None);

    if let Some(ref url) = t.repair_history_url {
        elements.push(AasSubmodelElement::Reference(AasReference {
            id_short: "repairHistoryUrl".into(),
            value: url.clone(),
            semantic_id: None,
        }));
    }

    if let Some(ref svhcs) = t.svhc_substances {
        let items = svhcs
            .iter()
            .enumerate()
            .map(|(i, s)| {
                let mut collection = svhc_substance_element(
                    i,
                    &s.cas_number,
                    &s.substance_name,
                    s.concentration_pct,
                    s.location_in_product.as_deref(),
                );
                if let Some(ref scip) = s.scip_notification_id {
                    collection
                        .value
                        .push(string_property("scipNotificationId", scip, None, None));
                }
                AasSubmodelElement::SubmodelElementCollection(collection)
            })
            .collect();
        elements.push(AasSubmodelElement::SubmodelElementCollection(
            AasCollection {
                id_short: "svhcSubstances".into(),
                value: items,
                semantic_id: None,
            },
        ));
    }

    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:textile-material-declaration"),
        id_short: "TextileMaterialDeclaration".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::TEXTILE_MATERIAL)),
        submodel_elements: elements,
    }
}
