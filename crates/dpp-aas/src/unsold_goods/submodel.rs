//! Unsold-goods reports as an AAS submodel.
//!
//! One submodel per disclosure, carrying the Annex I header and a flattened
//! element per line. It is not a passport submodel: a disclosure is not the data
//! of a passport, so this mapper is reached through
//! [`build_unsold_goods_environment`](super::build_unsold_goods_environment) and
//! not through the product-group dispatch.

use dpp_domain::unsold_goods::{DisclosureScope, UnsoldGoodsReport};

use crate::model::{AasSemId, AasSubmodel};
use crate::property::{double_property, opt_enum_wire_str, string_property};
use crate::semantic_ids;

/// One submodel per disclosure, carrying the Annex I header and a flattened
/// element per line.
///
/// AAS `SubmodelElement`s are a flat list here rather than a
/// `SubmodelElementCollection` per line — the disclosure has a repeating body
/// (Annex I: "additional lines may be added as necessary") and this projection
/// indexes it. A reader wanting the structured record should take the JSON;
/// this exists for toolchains that only speak AAS.
pub(super) fn build_unsold_goods_submodel(r: &UnsoldGoodsReport, id: &str) -> AasSubmodel {
    let mut elements = vec![
        string_property("entityName", &r.entity.name, None),
        string_property("entityIdentifier", r.entity.identifier.value(), None),
        string_property(
            "disclosureScope",
            match r.entity.scope {
                DisclosureScope::Standalone => "standalone",
                DisclosureScope::Consolidated { .. } => "consolidated",
                // `DisclosureScope` is `#[non_exhaustive]`: a scope this build
                // has no name for is projected as unknown rather than guessed.
                _ => "unknown",
            },
            None,
        ),
        string_property(
            "financialYearStart",
            &r.financial_year.start.to_string(),
            None,
        ),
        string_property("financialYearEnd", &r.financial_year.end.to_string(), None),
        double_property("totalWeightKg", r.total_weight_kg() as f64, None),
        double_property("totalUnits", r.total_units() as f64, None),
    ];

    for (i, line) in r.lines.iter().enumerate() {
        let reason_str = opt_enum_wire_str(&line.reason);
        elements.push(string_property(
            &format!("line{i}CnCategories"),
            &line
                .cn_categories
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(","),
            None,
        ));
        elements.push(string_property(
            &format!("line{i}Description"),
            &line.description,
            None,
        ));
        elements.push(double_property(
            &format!("line{i}WeightKg"),
            line.weight_kg.value as f64,
            None,
        ));
        elements.push(double_property(
            &format!("line{i}Units"),
            line.units_discarded.value as f64,
            None,
        ));
        // Absent, not empty, where no derogation applies (Annex I note (h):
        // "where applicable").
        if let Some(reason_str) = reason_str {
            elements.push(string_property(
                &format!("line{i}Reason"),
                &reason_str,
                None,
            ));
        }
        // Derived, per Annex I note (i) — never a stored field, but a reader of
        // the projection needs it without recomputing.
        elements.push(double_property(
            &format!("line{i}TotalDestructionPct"),
            f64::from(line.treatment.total_destruction_pct()),
            None,
        ));
    }

    elements.push(string_property("measuresTaken", &r.measures_taken, None));
    elements.push(string_property(
        "measuresPlanned",
        &r.measures_planned,
        None,
    ));

    AasSubmodel {
        id: id.to_owned(),
        id_short: "UnsoldGoods".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::UNSOLD_GOODS_REPORT)),
        submodel_elements: elements,
    }
}
