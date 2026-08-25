//! Unsold-goods reports as an AAS submodel.
//!
//! # Reachable through the library, not over HTTP
//!
//! An AAS shell's `globalAssetId` is built from the GTIN, and an unsold-goods
//! report has none — it declares a *volume of goods disposed of in a reporting
//! period*, not a trade item, so `ProductGroupData::gtin()` returns `None` for it by
//! design. A content-negotiated HTTP door that serves AAS therefore has nothing
//! to identify the asset with and answers `406`: this mapper never runs for a
//! request over the wire, and no amount of fixing the door changes that.
//!
//! It is kept, rather than deleted, because the constraint is the *shell's*,
//! not the submodel's. Callers that supply their own asset identity — a file
//! export, an AASX package assembled for a reporting authority, a caller
//! building an Environment around a non-trade-item asset — get a correct
//! submodel from it today. `build_aas_from_passport` takes `gtin` as a
//! parameter precisely so the identity is the caller's to decide.
//!
//! So: if you are here because an HTTP request for
//! `application/aas+json` returned `406` on an unsold-goods passport, that is
//! the designed answer and not a defect in this file. Inventing a
//! `globalAssetId` to make the door respond would put a fabricated trade-item
//! identifier into a document an integrator's toolchain treats as authoritative.

use dpp_domain::product_group::{DisclosureScope, UnsoldGoodsReport};

use crate::model::{AasSemId, AasSubmodel};
use crate::property::{double_property, enum_wire_str, string_property};
use crate::semantic_ids;

/// One submodel per disclosure, carrying the Annex I header and a flattened
/// element per line.
///
/// AAS `SubmodelElement`s are a flat list here rather than a
/// `SubmodelElementCollection` per line — the disclosure has a repeating body
/// (Annex I: "additional lines may be added as necessary") and this projection
/// indexes it. A reader wanting the structured record should take the JSON;
/// this exists for toolchains that only speak AAS.
pub(super) fn build_unsold_goods_submodel(r: &UnsoldGoodsReport, passport_id: &str) -> AasSubmodel {
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
        let reason_str = enum_wire_str(&line.reason);
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
        elements.push(string_property(
            &format!("line{i}Reason"),
            &reason_str,
            None,
        ));
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
        id: format!("urn:odal-node:dpp:{passport_id}:unsold-goods"),
        id_short: "UnsoldGoods".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::UNSOLD_GOODS_REPORT)),
        submodel_elements: elements,
    }
}
