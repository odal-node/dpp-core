use dpp_domain::domain::sector::UnsoldGoodsReport;

use crate::aas::model::{AasSemId, AasSubmodel};
use crate::aas::property::{double_property, enum_wire_str, string_property};
use crate::aas::semantic_ids;

pub(super) fn build_unsold_goods_submodel(r: &UnsoldGoodsReport, passport_id: &str) -> AasSubmodel {
    let reason_str = enum_wire_str(&r.reason);
    let destination_str = enum_wire_str(&r.destination);
    let mut elements = vec![
        string_property("reportingPeriod", &r.reporting_period, None, None),
        double_property("volumeKg", r.volume_kg, None, Some("kg")),
        string_property("productCategory", &r.product_category, None, None),
        string_property("reason", &reason_str, None, None),
        string_property("destination", &destination_str, None, None),
        string_property("countryOfDisposal", &r.country_of_disposal, None, None),
    ];
    if let Some(ref v) = r.destruction_justification {
        elements.push(string_property("destructionJustification", v, None, None));
    }
    if let Some(ref v) = r.operator_name {
        elements.push(string_property("operatorName", v, None, None));
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
