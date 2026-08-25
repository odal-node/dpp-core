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

use dpp_domain::domain::product_group::UnsoldGoodsReport;

use crate::model::{AasSemId, AasSubmodel};
use crate::property::{double_property, enum_wire_str, string_property};
use crate::semantic_ids;

pub(super) fn build_unsold_goods_submodel(r: &UnsoldGoodsReport, passport_id: &str) -> AasSubmodel {
    let reason_str = enum_wire_str(&r.reason);
    let destination_str = enum_wire_str(&r.destination);
    let mut elements = vec![
        string_property("reportingPeriod", &r.reporting_period, None),
        double_property("volumeKg", r.volume_kg, None),
        string_property("productCategory", &r.product_category, None),
        string_property("reason", &reason_str, None),
        string_property("destination", &destination_str, None),
        string_property("countryOfDisposal", &r.country_of_disposal, None),
    ];
    if let Some(ref v) = r.destruction_justification {
        elements.push(string_property("destructionJustification", v, None));
    }
    if let Some(ref v) = r.operator_name {
        elements.push(string_property("operatorName", v, None));
    }
    AasSubmodel {
        id: format!("urn:odal-node:dpp:{passport_id}:unsold-goods"),
        id_short: "UnsoldGoods".into(),
        model_type: "Submodel".into(),
        kind: "Instance".into(),
        semantic_id: Some(AasSemId::external(semantic_ids::UNSOLD_GOODS_REPORT)),
        submodel_elements: elements,
    }
}
